#![no_std]
#![no_main]
extern crate panic_semihosting;

use axis::Axis;
use cortex_m::asm::delay;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use filters::delta_pass::DeltaPass;
use filters::lerp::Lerp;
use filters::mean::Mean;
use stm32f1xx_hal::adc;
use stm32f1xx_hal::usb::Peripheral;
use stm32f1xx_hal::{pac, prelude::*};
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usbd_human_interface_device::device::joystick::JoystickReport;
use usbd_human_interface_device::usb_class::UsbHidClassBuilder;
use usbd_human_interface_device::UsbHidError;

mod filters;
mod axis;

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

    assert!(clocks.usbclk_valid());
    let mut gpioa = dp.GPIOA.split();
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low();
    delay(clocks.sysclk().raw() / 100);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };
    let usb_bus = stm32f1xx_hal::usb::UsbBus::new(usb);
    let mut joy = UsbHidClassBuilder::new()
        .add_device(usbd_human_interface_device::device::joystick::JoystickConfig::default())
        .build(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27de))
        .manufacturer("Blue Skies")
        .product("Trotllik")
        .serial_number("TEST")
        .device_class(0x03)
        .build();

    let mut adc1 = adc::Adc::adc1(dp.ADC1, clocks);
    
    let mut throttle_pot = gpioa.pa7.into_analog(&mut gpioa.crl);
    let mut prop_pot = gpioa.pa6.into_analog(&mut gpioa.crl);
    let mut mixture_pot = gpioa.pa5.into_analog(&mut gpioa.crl);
    
    let mut throttle_axis: Axis<2> = Axis::new(3300, 4090, true);
    throttle_axis.add_filter(Mean::new());
    throttle_axis.add_filter(DeltaPass::new(5));
    let mut prop_axis: Axis<2> = Axis::new(3350, 4090, true);
    prop_axis.add_filter(Mean::new());
    throttle_axis.add_filter(DeltaPass::new(5));

    let mut mixture_axis: Axis<2> = Axis::new(3300, 4090, true);
    mixture_axis.add_filter(Mean::new());
    throttle_axis.add_filter(DeltaPass::new(5));

    loop {
        let report = JoystickReport {
            x: throttle_axis.output_ranged(0, 1024),
            y: prop_axis.output_ranged(0, 1024),
            z: mixture_axis.output_ranged(0, 1024),
            buttons: 0
        };

        //hprintln!("{} {} {}", report.x, report.y, report.z);
        delay(1);
        match joy.device().write_report(&report) {
            Err(UsbHidError::WouldBlock) => {}
            Ok(_) => {}
            Err(e) => {
                core::panic!("Failed to write joystick report: {:?}", e)
            }
        }

        if !usb_dev.poll(&mut [&mut joy]) {
            throttle_axis.process(adc1.read(&mut throttle_pot).unwrap_or_default()); 
            prop_axis.process(adc1.read(&mut prop_pot).unwrap_or_default());
            mixture_axis.process(adc1.read(&mut mixture_pot).unwrap_or_default());
        }
    }
}
