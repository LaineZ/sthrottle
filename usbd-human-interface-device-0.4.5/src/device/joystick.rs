//!HID joystick
use crate::usb_class::prelude::*;
use core::default::Default;
use fugit::ExtU32;
use packed_struct::prelude::*;
use usb_device::bus::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;

#[rustfmt::skip]
pub const DEFAULT_JOYSTICK_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)         5,   1
    0x09, 0x04, // Usage (Joystick)                     9,   4
    0xa1, 0x01, // Collection (Application)             161, 1
    0x09, 0x01, //   Usage Page (Pointer)               9,   1
    0xa1, 0x00, //   Collection (Physical)              161, 0
    0x09, 0x30, //     Usage (X)                        9,   48
    0x09, 0x31, //     Usage (Y)                        9,   49
    0x09, 0x32, //     Usage (Z)
    0x15, 0x00, //     Logical Minimum (0)              21,  0
    0x26, 0x00, 0x04, // Logical Maximum (1024)        38,  0,  16
    0x75, 0x10, //     Report Size (16)                 117, 16
    0x95, 0x03, //     Report Count (3)                 149, 3
    0x81, 0x02, //     Input (Data, Variable, Absolute) 129, 2
    0xc0,       //   End Collection                     192
    0x05, 0x09, //   Usage Page (Button)                5,   9
    0x19, 0x01, //   Usage Minimum (1)                  25,  1
    0x29, 0x08, //   Usage Maximum (8)                  41,  8
    0x15, 0x00, //   Logical Minimum (0)                21,  0
    0x25, 0x01, //   Logical Maximum (1)                37,  1
    0x75, 0x01, //   Report Size (1)                    117, 1
    0x95, 0x08, //   Report Count (8)                   149, 8
    0x81, 0x02, //   Input (Data, Variable, Absolute)   129, 2
    0xc0        // End Collection                       192
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, PackedStruct)]
#[packed_struct(endian = "lsb", size_bytes = "7")]
pub struct JoystickReport {
    #[packed_field]
    pub x: u16,
    #[packed_field]
    pub y: u16,
    #[packed_field]
    pub z: u16,
    #[packed_field]
    pub buttons: u8,
}

pub struct Joystick<'a, B: UsbBus> {
    interface: Interface<'a, B, InBytes8, OutNone, ReportSingle>,
}

impl<'a, B: UsbBus> Joystick<'a, B> {
    pub fn write_report(&mut self, report: &JoystickReport) -> Result<(), UsbHidError> {
        let data = report.pack().map_err(|_| {
            error!("Error packing JoystickReport");
            UsbHidError::SerializationError
        })?;
        self.interface
            .write_report(&data)
            .map(|_| ())
            .map_err(UsbHidError::from)
    }
}

impl<'a, B: UsbBus> DeviceClass<'a> for Joystick<'a, B> {
    type I = Interface<'a, B, InBytes8, OutNone, ReportSingle>;

    fn interface(&mut self) -> &mut Self::I {
        &mut self.interface
    }

    fn reset(&mut self) {}

    fn tick(&mut self) -> Result<(), UsbHidError> {
        Ok(())
    }
}

pub struct JoystickConfig<'a> {
    interface: InterfaceConfig<'a, InBytes8, OutNone, ReportSingle>,
}

impl<'a> JoystickConfig<'a> {
    pub fn new_with_descriptor(descriptior: &'a [u8]) -> Self {
         Self::new(
            unwrap!(unwrap!(InterfaceBuilder::new(descriptior))
                .boot_device(InterfaceProtocol::None)
                .description("Joystick")
                .in_endpoint(10.millis()))
            .without_out_endpoint()
            .build(),
        )
    }
}

impl<'a> Default for JoystickConfig<'a> {
    #[must_use]
    fn default() -> Self {
        Self::new(
            unwrap!(unwrap!(InterfaceBuilder::new(DEFAULT_JOYSTICK_DESCRIPTOR))
                .boot_device(InterfaceProtocol::None)
                .description("Joystick")
                .in_endpoint(10.millis()))
            .without_out_endpoint()
            .build(),
        )
    }
}

impl<'a> JoystickConfig<'a> {
    #[must_use]
    pub fn new(interface: InterfaceConfig<'a, InBytes8, OutNone, ReportSingle>) -> Self {
        Self { interface }
    }
}

impl<'a, B: UsbBus + 'a> UsbAllocatable<'a, B> for JoystickConfig<'a> {
    type Allocated = Joystick<'a, B>;

    fn allocate(self, usb_alloc: &'a UsbBusAllocator<B>) -> Self::Allocated {
        Self::Allocated {
            interface: Interface::new(usb_alloc, self.interface),
        }
    }
}
