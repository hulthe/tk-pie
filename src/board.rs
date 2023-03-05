use embassy_rp::peripherals::*;

/// Pinouts for the ItsyBitsy
#[allow(dead_code)]
pub struct Board {
    pub a0: PIN_26,
    pub a1: PIN_27,
    pub a2: PIN_28,
    pub a3: PIN_29,
    pub d24: PIN_24,
    pub d25: PIN_25,
    pub sck: PIN_18,
    pub mosi: PIN_19,
    pub miso: PIN_20,
    pub d2: PIN_12,
    pub d3: PIN_5,
    pub d4: PIN_4,
    pub rx: PIN_1,
    pub tx: PIN_0,
    pub sda: PIN_2,
    pub scl: PIN_3,
    pub d5: PIN_14,
    pub d7: PIN_6,
    pub d9: PIN_7,
    pub d10: PIN_8,
    pub d11: PIN_9,
    pub d12: PIN_10,
    pub d13: PIN_11,
    pub neopixel: PIN_17,
    pub neopixel_power: PIN_16,
}
