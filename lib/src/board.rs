use crate::keyboard::KeyboardConfig;
use crate::ws2812::Ws2812;
use embassy_rp::{peripherals::*, Peri, Peripherals};

/// Pinouts for the ItsyBitsy
#[allow(dead_code, non_snake_case)]
pub struct Board {
    pub USB: Peri<'static, USB>,
    pub UART0: Peri<'static, UART0>,
    pub UART1: Peri<'static, UART1>,
    pub PIO0: Peri<'static, PIO0>,
    pub PIO1: Peri<'static, PIO1>,
    pub DMA_CH0: Peri<'static, DMA_CH0>,
    pub DMA_CH1: Peri<'static, DMA_CH1>,
    pub DMA_CH2: Peri<'static, DMA_CH2>,
    pub DMA_CH3: Peri<'static, DMA_CH3>,
    pub DMA_CH4: Peri<'static, DMA_CH4>,
    pub DMA_CH5: Peri<'static, DMA_CH5>,
    pub DMA_CH6: Peri<'static, DMA_CH6>,
    pub DMA_CH7: Peri<'static, DMA_CH7>,
    pub DMA_CH8: Peri<'static, DMA_CH8>,
    pub DMA_CH9: Peri<'static, DMA_CH9>,
    pub DMA_CH10: Peri<'static, DMA_CH10>,
    // pins
    pub a0: Peri<'static, PIN_26>,
    pub a1: Peri<'static, PIN_27>,
    pub a2: Peri<'static, PIN_28>,
    pub a3: Peri<'static, PIN_29>,
    pub d24: Peri<'static, PIN_24>,
    pub d25: Peri<'static, PIN_25>,
    pub sck: Peri<'static, PIN_18>,
    pub mosi: Peri<'static, PIN_19>,
    pub miso: Peri<'static, PIN_20>,
    pub d2: Peri<'static, PIN_12>,
    pub d3: Peri<'static, PIN_5>,
    pub d4: Peri<'static, PIN_4>,
    pub rx: Peri<'static, PIN_1>,
    pub tx: Peri<'static, PIN_0>,
    pub sda: Peri<'static, PIN_2>,
    pub scl: Peri<'static, PIN_3>,
    pub d5: Peri<'static, PIN_14>,
    pub d7: Peri<'static, PIN_6>,
    pub d9: Peri<'static, PIN_7>,
    pub d10: Peri<'static, PIN_8>,
    pub d11: Peri<'static, PIN_9>,
    pub d12: Peri<'static, PIN_10>,
    pub d13: Peri<'static, PIN_11>,
    pub neopixel: Peri<'static, PIN_17>,
    pub neopixel_power: Peri<'static, PIN_16>,
}

/// A [Board] with the keyboard pins mapped.
#[allow(dead_code, non_snake_case)]
pub struct MappedBoard {
    pub USB: Peri<'static, USB>,
    pub UART0: Peri<'static, UART0>,
    pub UART1: Peri<'static, UART1>,
    pub keyboard: KeyboardConfig,
    pub rx: Peri<'static, PIN_1>,
    pub tx: Peri<'static, PIN_0>,
    /// The pin controlling the onboard ItsyBitsy neopixel
    pub neopixel: Ws2812<PIO0>,

    /// The pin controlling power for the onboard ItsyBitsy neopixel
    pub neopixel_power: Peri<'static, PIN_16>,
}

impl From<Peripherals> for Board {
    fn from(p: Peripherals) -> Self {
        Board {
            USB: p.USB,
            UART0: p.UART0,
            UART1: p.UART1,
            PIO0: p.PIO0,
            PIO1: p.PIO1,
            DMA_CH0: p.DMA_CH0,
            DMA_CH1: p.DMA_CH1,
            DMA_CH2: p.DMA_CH2,
            DMA_CH3: p.DMA_CH3,
            DMA_CH4: p.DMA_CH4,
            DMA_CH5: p.DMA_CH5,
            DMA_CH6: p.DMA_CH6,
            DMA_CH7: p.DMA_CH7,
            DMA_CH8: p.DMA_CH8,
            DMA_CH9: p.DMA_CH9,
            DMA_CH10: p.DMA_CH10,

            // pins
            a0: p.PIN_26,
            a1: p.PIN_27,
            a2: p.PIN_28,
            a3: p.PIN_29,
            d24: p.PIN_24,
            d25: p.PIN_25,
            sck: p.PIN_18,
            mosi: p.PIN_19,
            miso: p.PIN_20,
            d2: p.PIN_12,
            d3: p.PIN_5,
            d4: p.PIN_4,
            rx: p.PIN_1,
            tx: p.PIN_0,
            sda: p.PIN_2,
            scl: p.PIN_3,
            d5: p.PIN_14,
            d7: p.PIN_6,
            d9: p.PIN_7,
            d10: p.PIN_8,
            d11: p.PIN_9,
            d12: p.PIN_10,
            d13: p.PIN_11,
            neopixel: p.PIN_17,
            neopixel_power: p.PIN_16,
        }
    }
}
