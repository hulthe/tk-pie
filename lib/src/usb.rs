use embassy_executor::Spawner;
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_usb::{Builder, Config, Handler, UsbDevice};
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use crate::{interrupts::Irqs, keyboard::KbEvents, uart::UART_USB_EVENTS_OUT, util::CS};

pub mod keyboard;
pub mod logger;

pub const MAX_PACKET_SIZE: u8 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsbEvent {
    Enabled(bool),
    Suspended(bool),
    Configured(bool),
    Addressed(u8),
    Reset,
}

pub type UsbEventChannel = PubSubChannel<CS, UsbEvent, 8, 24, 0>;
pub static USB_EVENTS: UsbEventChannel = UsbEventChannel::new();

struct State {
    device_descriptor: [u8; 256],
    config_descriptor: [u8; 256],
    bos_descriptor: [u8; 256],
    msos_descriptor: [u8; 256],
    control_buf: [u8; 64],
    handler: UsbHandler,
}

static STATE: StaticCell<State> = StaticCell::new();

pub async fn setup_logger_and_keyboard(usb: USB, events: KbEvents) {
    let mut builder = builder(usb);

    //logger::setup(&mut builder).await;

    keyboard::setup(&mut builder, events).await;

    log::info!("building usb device");
    let usb = builder.build();
    log::info!("spawning usb task");
    Spawner::for_current_executor().await.must_spawn(run(usb));
}

pub fn builder(usb: USB) -> Builder<'static, Driver<'static, USB>> {
    // calling init here can't panic because this function can't be called
    // twice since we are taking ownership of the only USB peripheral.
    let state = STATE.init_with(|| State {
        device_descriptor: [0; 256],
        config_descriptor: [0; 256],
        bos_descriptor: [0; 256],
        msos_descriptor: [0; 256],
        control_buf: [0; 64],
        handler: UsbHandler,
    });

    // Create embassy-usb Config
    let mut config = Config::new(0xb00b, 0x1355);
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

    let driver = Driver::new(usb, Irqs);

    let mut builder = Builder::new(
        driver,
        config,
        &mut state.device_descriptor,
        &mut state.config_descriptor,
        &mut state.bos_descriptor,
        &mut state.msos_descriptor,
        &mut state.control_buf,
    );

    builder.handler(&mut state.handler);

    builder
}

#[embassy_executor::task]
pub async fn run(mut device: UsbDevice<'static, Driver<'static, USB>>) {
    log::info!("running usb device");
    device.run().await
}

struct UsbHandler;

impl Handler for UsbHandler {
    fn enabled(&mut self, enabled: bool) {
        USB_EVENTS.publish_immediate(UsbEvent::Enabled(enabled));
        let _ = UART_USB_EVENTS_OUT.try_send(UsbEvent::Enabled(enabled));
        log::debug!("usb enabled({enabled})");
    }

    fn reset(&mut self) {
        USB_EVENTS.publish_immediate(UsbEvent::Reset);
        let _ = UART_USB_EVENTS_OUT.try_send(UsbEvent::Reset);
        log::debug!("usb reset()");
    }

    fn addressed(&mut self, addr: u8) {
        USB_EVENTS.publish_immediate(UsbEvent::Addressed(addr));
        let _ = UART_USB_EVENTS_OUT.try_send(UsbEvent::Addressed(addr));
        log::debug!("usb addressed({addr})");
    }

    fn configured(&mut self, configured: bool) {
        USB_EVENTS.publish_immediate(UsbEvent::Configured(configured));
        let _ = UART_USB_EVENTS_OUT.try_send(UsbEvent::Configured(configured));
        log::debug!("usb configured({configured})");
    }

    fn suspended(&mut self, suspended: bool) {
        USB_EVENTS.publish_immediate(UsbEvent::Suspended(suspended));
        let _ = UART_USB_EVENTS_OUT.try_send(UsbEvent::Suspended(suspended));
        log::debug!("usb suspended({suspended})");
    }
}
