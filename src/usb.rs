use embassy_rp::{interrupt, peripherals::USB, usb::Driver};
use embassy_usb::{Builder, Config, UsbDevice};
use static_cell::StaticCell;

pub mod keyboard;
pub mod logger;

pub const MAX_PACKET_SIZE: u8 = 64;

struct State {
    device_descriptor: [u8; 256],
    config_descriptor: [u8; 256],
    bos_descriptor: [u8; 256],
    control_buf: [u8; 64],
}

static STATE: StaticCell<State> = StaticCell::new();

pub fn builder(usb: USB) -> Builder<'static, Driver<'static, USB>> {
    let state = STATE.init(State {
        device_descriptor: [0; 256],
        config_descriptor: [0; 256],
        bos_descriptor: [0; 256],
        control_buf: [0; 64],
    });

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Tux");
    config.product = Some("Tangentbord1");
    config.serial_number = Some("42069");
    config.max_power = 100;
    config.max_packet_size_0 = MAX_PACKET_SIZE;

    // Required for windows compatiblity.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    let driver = Driver::new(usb, interrupt::take!(USBCTRL_IRQ));

    Builder::new(
        driver,
        config,
        &mut state.device_descriptor,
        &mut state.config_descriptor,
        &mut state.bos_descriptor,
        &mut state.control_buf,
    )
}

#[embassy_executor::task]
pub async fn run(mut device: UsbDevice<'static, Driver<'static, USB>>) {
    device.run().await
}
