#![no_std]
#![no_main]

pub mod button;

extern crate panic_semihosting;
use axis::{Axis, DynEffect};
use button::Button;
use cortex_m::asm::delay;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use heapless::Vec;
use stm32f1xx_hal::flash::{FlashSize, FlashWriter, SectorSize};
use stm32f1xx_hal::timer::{Channel, Tim3NoRemap};
use stm32f1xx_hal::usb::Peripheral;
use stm32f1xx_hal::{pac, adc, prelude::*};
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usbd_human_interface_device::device::joystick::JoystickReport;
use usbd_human_interface_device::usb_class::UsbHidClassBuilder;
use usbd_human_interface_device::UsbHidError;

const FLASH_BASE: usize = 0x0800_0000;
const LAST_PAGE_ADDRESS: usize = 0x0800_F800;

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

#[repr(C)]
struct Config {
    throttle_axis_min: u16,
    throttle_axis_max: u16,
    prop_axis_min: u16,
    prop_axis_max: u16,
    mixture_axis_min: u16,
    mixture_axis_max: u16
}

impl Config {
    fn new(writer: &FlashWriter) -> Self {
        // TODO: Proper error handling
        // FIXME: Reinterpret struct instead of extracting fields manually?
        let base_offset = LAST_PAGE_ADDRESS - FLASH_BASE;  // 0xF800
        let values = writer.read(base_offset as u32, 12).unwrap();
        let data: Vec<u16, 6> = values
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();


        Self {
            throttle_axis_min: data[0],
            throttle_axis_max: data[1],
            prop_axis_min: data[2],
            prop_axis_max: data[3],
            mixture_axis_min: data[4],
            mixture_axis_max: data[5],
        }
    }

    fn save(&self, writer: &mut FlashWriter) {
        // TODO: Proper error handling
        // FIXME: Reinterpret struct instead of extracting fields manually?
        let base_offset = LAST_PAGE_ADDRESS - FLASH_BASE;  // 0xF800
        let data: [u16; 6] = [self.throttle_axis_min, self.throttle_axis_max, self.prop_axis_min, self.prop_axis_max, self.mixture_axis_min, self.mixture_axis_max];
        writer.page_erase(base_offset as u32);

        hprintln!("writing...");
        for (i, val) in data.iter().enumerate() {
            let addr = base_offset as u32 + i as u32 * 2;
            writer.write(addr, &val.to_le_bytes()).unwrap();
        }

        hprintln!("done!");
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            throttle_axis_min: 3300,
            throttle_axis_max: 4090,
            prop_axis_min: 3300,
            prop_axis_max: 4090,
            mixture_axis_min: 3300,
            mixture_axis_max: 4090,
        }
    }
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

    // flash writer
    let mut writer = flash.writer(SectorSize::Sz1K, FlashSize::Sz64K);

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
    //let mut calibration_data = CalibrationData::default();

    let mut throttle_axis = Axis::new(0, 4096, true);
    let mut prop_axis = Axis::new(0, 4096, true);
    let mut mixture_axis = Axis::new(0, 4096, true);
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
               if calibrate_button.pressed() { 
                   state = Stage::CalibrationStageHigh;
               };
            },
            Stage::CalibrationStageHigh => {
               pwm.set_duty(Channel::C3, pwm.get_max_duty() / 2);
               if calibrate_button.pressed() { 
                   state = Stage::Normal;
                   pwm.set_duty(Channel::C3, 0);
               };
            },
        };
    }
}
