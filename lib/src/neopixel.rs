/*
use embassy_rp::{
    gpio::{AnyPin, Drive, SlewRate},
    peripherals::{DMA_CH0, PIO0},
    pio::{FifoJoin, PioPeripheral, PioStateMachine, ShiftDirection},
    pio_instr_util,
    relocate::RelocatedProgram,
    PeripheralRef,
};
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn test(pio: PIO0, pin: AnyPin, dma: DMA_CH0) {
    let (_common, mut sm, ..) = pio.split();
    let mut dma = PeripheralRef::new(dma);

    let pio_program = pio_proc::pio_file!("src/neopixel.pio");

    let relocated = RelocatedProgram::new(&pio_program.program);
    sm.write_instr(relocated.origin() as usize, relocated.code());
    pio_instr_util::exec_jmp(&mut sm, relocated.origin());

    let pin = sm.make_pio_pin(pin);
    sm.set_set_pins(&[&pin]);
    sm.set_sideset_base_pin(&pin);
    sm.set_sideset_count(1);

    // Clock config
    // TODO CLOCK_FREQ should come from embassy_rp
    const CLOCK_FREQ: u32 = 125_000_000;
    const WS2812_FREQ: u32 = 800_000;
    const CYCLES_PER_BIT: u32 = 16;

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

    sm.set_autopull(true);
    sm.set_fifo_join(FifoJoin::TxOnly);
    sm.set_pull_threshold(8); // 24?
    sm.set_out_shift_dir(ShiftDirection::Left);

    sm.set_enable(true);

    log::info!("wrap: {:?}", sm.get_wrap());
    log::info!("addr: {:?}", sm.get_addr());
    log::info!("sideset_base: {:?}", sm.get_sideset_base());
    log::info!("sideset_count: {:?}", sm.get_sideset_count());
    log::info!("in_base: {:?}", sm.get_in_base());
    log::info!("jmp_pin: {:?}", sm.get_jmp_pin());
    log::info!("set_range: {:?}", sm.get_set_range());
    log::info!("out_range: {:?}", sm.get_out_range());
    log::info!("pull_threshold: {:?}", sm.get_pull_threshold());
    log::info!("push_threshold: {:?}", sm.get_push_threshold());

    //sm = rp2pio.StateMachine(
    //    assembled,
    //    frequency=12_800_000,  # to get appropriate sub-bit times in PIO program
    //    first_sideset_pin=NEOPIXEL,
    //    auto_pull=True,
    //    out_shift_right=False,
    //    pull_threshold=8,
    //)

    loop {
        log::info!("sending dma");

        sm.dma_push(dma.reborrow(), &[0x0a, 0x00, 0x00]).await;
        Timer::after(Duration::from_millis(500)).await;
        sm.dma_push(dma.reborrow(), &[0x00, 0x0a, 0x00]).await;
        Timer::after(Duration::from_millis(500)).await;
        sm.dma_push(dma.reborrow(), &[0x00, 0x00, 0x0a]).await;
        Timer::after(Duration::from_millis(500)).await;

        //sm0.set_enable(true);
    }
}

#[embassy_executor::task]
pub async fn test_blink(pio: PIO0, pin: AnyPin) {
    log::info!("test blink hehe");
    let (_, mut sm, ..) = pio.split();
    // Setup sm2

    // blink
    let prg = pio_proc::pio_file!("src/blink.pio");

    let relocated = RelocatedProgram::new(&prg.program);
    let out_pin = sm.make_pio_pin(pin);
    let pio_pins = [&out_pin];
    sm.set_set_pins(&pio_pins);
    sm.set_set_range(25, 1);

    sm.write_instr(relocated.origin() as usize, relocated.code());
    pio_instr_util::exec_jmp(&mut sm, relocated.origin());
    // sm.set_clkdiv((65535 << 8) + 255 as u32);
    // sm.set_clkdiv(0);

    let pio::Wrap { source, target } = relocated.wrap();
    sm.set_wrap(source, target);

    //     sm.set_clkdiv((125e6 / 20.0 / 2e2 * 256.0) as u32);
    sm.set_enable(true);
    // sm.wait_push().await as i32;
    // sm.push_tx(1);
    sm.wait_push(125_000_000).await;
    log::info!("started");

    loop {
        sm.wait_irq(3).await;
        log::info!("did it!");
    }
}
*/
