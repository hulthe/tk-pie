use embassy_rp::{
    bind_interrupts,
    peripherals::{UART0, USB},
};

bind_interrupts! {
    pub struct Irqs {
        UART0_IRQ => embassy_rp::uart::BufferedInterruptHandler<UART0>;
        USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
    }
}
