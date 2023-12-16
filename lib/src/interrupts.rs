use embassy_rp::{
    bind_interrupts,
    peripherals::{PIO0, PIO1, UART0, USB},
};

bind_interrupts! {
    pub struct Irqs {
        UART0_IRQ => embassy_rp::uart::BufferedInterruptHandler<UART0>;
        USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
        PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
        PIO1_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO1>;
    }
}
