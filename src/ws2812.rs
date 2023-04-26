use core::fmt::{self, Debug};
use core::mem::transmute;
use core::ops::Div;

use embassy_rp::dma::{self, AnyChannel};
use embassy_rp::pio::{
    FifoJoin, PioInstance, PioPeripheral, PioStateMachine, PioStateMachineInstance, ShiftDirection,
    SmInstanceBase,
};
use embassy_rp::pio_instr_util;
use embassy_rp::relocate::RelocatedProgram;
use embassy_rp::{gpio, PeripheralRef};

pub struct Ws2812<P: PioInstance> {
    sm: PioStateMachineInstance<P, SmInstanceBase<0>>,
    dma: PeripheralRef<'static, AnyChannel>,
}

/// An Rgb value that can be safely transmuted to u32 for use with Ws2812.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rgb(u32);

impl<P: PioInstance> Ws2812<P> {
    pub fn new<PP: PioPeripheral<Pio = P>>(
        pio: PP,
        dma: impl dma::Channel,
        pin: gpio::AnyPin,
    ) -> Self {
        let (_, mut sm, ..) = pio.split();
        // prepare the PIO program
        let side_set = pio::SideSet::new(false, 1, false);
        let mut a: pio::Assembler<32> = pio::Assembler::new_with_side_set(side_set);

        const T1: u8 = 2; // start bit
        const T2: u8 = 5; // data bit
        const T3: u8 = 3; // stop bit
        const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32;

        let mut wrap_target = a.label();
        let mut wrap_source = a.label();
        let mut do_zero = a.label();
        a.set_with_side_set(pio::SetDestination::PINDIRS, 1, 0);
        a.bind(&mut wrap_target);
        // Do stop bit
        a.out_with_delay_and_side_set(pio::OutDestination::X, 1, T3 - 1, 0);
        // Do start bit
        a.jmp_with_delay_and_side_set(pio::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);
        // Do data bit = 1
        a.jmp_with_delay_and_side_set(pio::JmpCondition::Always, &mut wrap_target, T2 - 1, 1);
        a.bind(&mut do_zero);
        // Do data bit = 0
        a.nop_with_delay_and_side_set(T2 - 1, 0);
        a.bind(&mut wrap_source);

        let prg = a.assemble_with_wrap(wrap_source, wrap_target);

        let relocated = RelocatedProgram::new(&prg);
        sm.write_instr(relocated.origin() as usize, relocated.code());
        pio_instr_util::exec_jmp(&mut sm, relocated.origin());

        // Pin config
        let out_pin = sm.make_pio_pin(pin);
        sm.set_set_pins(&[&out_pin]);
        sm.set_sideset_base_pin(&out_pin);
        sm.set_sideset_count(1);

        // Clock config
        // TODO CLOCK_FREQ should come from embassy_rp
        const CLOCK_FREQ: u32 = 125_000_000;
        const WS2812_FREQ: u32 = 800_000;

        let bit_freq = WS2812_FREQ * CYCLES_PER_BIT;
        let mut int = CLOCK_FREQ / bit_freq;
        let rem = CLOCK_FREQ - (int * bit_freq);
        let frac = (rem * 256) / bit_freq;
        // 65536.0 is represented as 0 in the pio's clock divider
        if int == 65536 {
            int = 0;
        }

        sm.set_clkdiv((int << 8) | frac);
        let pio::Wrap { source, target } = relocated.wrap();
        sm.set_wrap(source, target);

        // FIFO config
        sm.set_autopull(true);
        sm.set_fifo_join(FifoJoin::TxOnly);
        sm.set_pull_threshold(24);
        sm.set_out_shift_dir(ShiftDirection::Left);

        sm.set_enable(true);

        Self {
            sm,
            dma: PeripheralRef::new(dma.degrade()),
        }
    }

    pub async fn write(&mut self, colors: &[Rgb]) {
        let colors = Rgb::slice_as_u32s(colors);
        self.sm.dma_push(self.dma.reborrow(), colors).await;
    }
}

impl Rgb {
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(u32::from_be_bytes([g, r, b, 0]))
    }

    /// Get the red, green, and blue components of this Rgb.
    #[inline(always)]
    pub const fn components(&self) -> (u8, u8, u8) {
        let [g, r, b, _] = self.0.to_be_bytes();
        (r, g, b)
    }

    #[inline(always)]
    pub fn slice_as_u32s(rgbs: &[Rgb]) -> &[u32] {
        // SAFETY: Rgb contains only a u32, and is #[repr(transparent)]
        unsafe { transmute(rgbs) }
    }
}

impl Debug for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (r, g, b) = self.components();
        f.debug_tuple("Rgb").field(&r).field(&g).field(&b).finish()
    }
}

impl Div<u8> for Rgb {
    type Output = Rgb;

    fn div(self, d: u8) -> Self::Output {
        let (r, g, b) = self.components();
        Rgb::new(r / d, g / d, b / d)
    }
}
