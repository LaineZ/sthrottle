#![no_std]
#![no_main]

pub mod button;

extern crate panic_semihosting;
use axis::{Axis, DynEffect};
use button::Button;
use cortex_m::asm::delay;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use stm32f1xx_hal::timer::{Channel, Tim1FullRemap, Tim1NoRemap, Tim2FullRemap, Tim2NoRemap, Tim2PartialRemap2, Tim3NoRemap, Timer};
use stm32f1xx_hal::usb::Peripheral;
use stm32f1xx_hal::{pac, adc, prelude::*};
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usbd_human_interface_device::device::joystick::JoystickReport;
use usbd_human_interface_device::usb_class::UsbHidClassBuilder;
use usbd_human_interface_device::UsbHidError;

#[derive(PartialEq, Clone, Copy)]
/// Allowed stages to operate
/// TODO: Implement all stages
pub enum Stage {
    /// Default mode, 3 axes, one button for enable/disable reverse
    Normal,
    /// Multiplexes the thorttle axis. This stage made for workaround X-Plane input system which
    /// allows bind ONLY ONE action on axis, and for somre reason reverse thrust can be set for
    /// separate engines only
    NormalXplane,
    /// Calibration stage for minimum range for all axes
    CalibrationStageLow,
    /// Calibration stage for max range for all axes
    CalibrationStageHigh,
}


#[derive(Clone, Copy)]
struct CalibrationAxisData {
    min: u16,
    max: u16,
    reversed: bool,
}

impl Default for CalibrationAxisData {
    fn default() -> Self {
        Self {
            min: 3300,
            max: 4090,
            reversed: true,
        }
    }
}

#[derive(Clone, Default, Copy)]
struct CalibrationData {
    throttle_axis: CalibrationAxisData,
    prop_axis: CalibrationAxisData,
    mixture_axis: CalibrationAxisData,
}

struct IndicationLED<P: OutputPin> {
    pin: P,
    current_tick: u32
}

#[entry]
fn main() -> ! {
    let _cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .pclk1(24.MHz())
        .adcclk(2.MHz())
        .freeze(&mut flash.acr);

    let mut gpioa = dp.GPIOA.split();
    let mut gpiob = dp.GPIOB.split();
    let mut afio = dp.AFIO.constrain();

    let mut state = Stage::Normal;
    let mut chain: [DynEffect; 0] = [];

    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    let mut indication_led = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    let mut adc1 = adc::Adc::adc1(dp.ADC1, clocks);
    let mut pwm = dp.TIM3.pwm_hz::<Tim3NoRemap, _, _>(indication_led, &mut afio.mapr, 1.Hz(), &clocks);

    pwm.enable(Channel::C1);
    pwm.enable(Channel::C2);
    pwm.enable(Channel::C3);

    pwm.set_duty(Channel::C2, 0);

    let mut throttle_pot = gpioa.pa1.into_analog(&mut gpioa.crl);
    let mut prop_pot = gpioa.pa2.into_analog(&mut gpioa.crl);
    let mut mixture_pot = gpioa.pa3.into_analog(&mut gpioa.crl);
    let mut calibrate_pot = gpioa.pa4.into_analog(&mut gpioa.crl);

    // TODO: Load calibration data from flash
    let mut calibration_data = CalibrationData::default();

    let mut throttle_axis = Axis::new(calibration_data.throttle_axis.min, calibration_data.throttle_axis.max, true);
    let mut prop_axis = Axis::new(calibration_data.prop_axis.min, calibration_data.prop_axis.max, true);
    let mut mixture_axis = Axis::new(calibration_data.mixture_axis.min, calibration_data.mixture_axis.max, true);
    let mut calibrate_value = Axis::new(0, 4096, true);
    // TODO: Reverse button
    let mut calibrate_button = Button::new(gpiob.pb1.into_pull_up_input(&mut gpiob.crl)); 

    assert!(clocks.usbclk_valid());
    usb_dp.set_low();
    delay(clocks.sysclk().raw() / 100);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };

    let usb_bus = stm32f1xx_hal::usb::UsbBus::new(usb);
    let mut joystick = UsbHidClassBuilder::new()
        .add_device(usbd_human_interface_device::device::joystick::JoystickConfig::default())
        .build(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27de))
        .manufacturer("Blue Skies")
        .product("Trotllik")
        .serial_number("TEST")
        .device_class(0x03)
        .build();

    loop {
        calibrate_value.update(
            adc1.read(&mut calibrate_pot).unwrap_or_default(),
            core::iter::empty(),
        );
        let step_filter_factor = calibrate_value.output(5, 20);
        throttle_axis.step_filter_factor = step_filter_factor;
        prop_axis.step_filter_factor = step_filter_factor;
        mixture_axis.step_filter_factor = step_filter_factor;

        let throttle_readings = adc1.read(&mut throttle_pot).unwrap_or_default();
        let prop_readings = adc1.read(&mut prop_pot).unwrap_or_default();
        let mixture_readings = adc1.read(&mut mixture_pot).unwrap_or_default();

        match state {
            Stage::NormalXplane => todo!(),
            Stage::Normal => {
                let report = JoystickReport {
                    x: throttle_axis.output(0, 1024),
                    y: prop_axis.output(0, 1024),
                    z: mixture_axis.output(0, 1024),
                    buttons: 0,
                };

                cortex_m_semihosting::hprintln!("{}", step_filter_factor);
                delay(1);
                match joystick.device().write_report(&report) {
                    Err(UsbHidError::WouldBlock) => {}
                    Ok(_) => {}
                    Err(e) => {
                        core::panic!("Failed to write joystick report: {:?}", e)
                    }
                }

                if !usb_dev.poll(&mut [&mut joystick]) {
                    throttle_axis.update(
                        throttle_readings,
                        chain.iter_mut(),
                    );
                    prop_axis.update(
                        prop_readings,
                        chain.iter_mut(),
                    );
                    mixture_axis.update(
                        mixture_readings,
                        chain.iter_mut(),
                    );
                }
            }
            Stage::CalibrationStageLow => {
               pwm.set_duty(Channel::C3, pwm.get_max_duty() / 4);
               calibration_data.throttle_axis.min = throttle_readings;
               calibration_data.prop_axis.min = prop_readings;
               calibration_data.mixture_axis.min = mixture_readings;
               if calibrate_button.pressed() { 
                   state = Stage::CalibrationStageHigh;
               };
            },
            Stage::CalibrationStageHigh => {
               pwm.set_duty(Channel::C3, pwm.get_max_duty() / 2);
               calibration_data.throttle_axis.max = throttle_readings;
               calibration_data.prop_axis.max = prop_readings;
               calibration_data.mixture_axis.max = mixture_readings;
               if calibrate_button.pressed() { 
                   state = Stage::Normal;
                   pwm.set_duty(Channel::C3, 0);
               };
            },
        };
    }
}
