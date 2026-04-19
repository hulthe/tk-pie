use embassy_executor::{Executor, InterruptExecutor, SendSpawner, Spawner};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt;
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_time::Timer;
use static_cell::StaticCell;

use crate::{
    board::{Board, MappedBoard},
    lights::LightDriver,
    logger::{LogMultiplexer, LogOutput},
    rgb::Rgb,
    util::stall,
    {allocator, hemicom::uart, rtt, usb},
};

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_MED: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_LOW: StaticCell<Executor> = StaticCell::new();

#[derive(Copy, Clone)]
pub struct Spawners {
    /// High priority executor. Tasks spawned on this will preempt medium and low priority tasks.
    pub high: SendSpawner,

    /// Medium priority executor. Tasks spawned on this will preempt low priority tasks.
    pub med: SendSpawner,

    /// Lom priority executor. Tasks spawned on this will be preempted by higher priority tasks.
    pub low: Spawner,
}

#[interrupt]
unsafe fn SWI_IRQ_1() {
    EXECUTOR_HIGH.on_interrupt()
}

#[interrupt]
unsafe fn SWI_IRQ_0() {
    EXECUTOR_MED.on_interrupt()
}

pub type InitFn = fn(Board) -> MappedBoard;

pub fn run(init: InitFn) -> ! {
    let rtt_logger = rtt::init_rtt_logger();

    allocator::init();

    let p = embassy_rp::init(Default::default());
    let board = Board::from(p);

    interrupt::SWI_IRQ_1.set_priority(Priority::P2);
    let spawner_high = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_1);

    interrupt::SWI_IRQ_0.set_priority(Priority::P3);
    let spawner_med = EXECUTOR_MED.start(interrupt::SWI_IRQ_0);

    let executor = EXECUTOR_LOW.init(Executor::new());
    executor.run(|spawner| {
        let spawners = Spawners {
            high: spawner_high,
            med: spawner_med,
            low: spawner,
        };

        spawner.must_spawn(main_task(board, init, rtt_logger, spawners));
    });
}

#[embassy_executor::task]
async fn main_task(
    board: Board,
    init: InitFn,
    rtt_logger: &'static dyn LogOutput,
    spawners: Spawners,
) {
    let mut board: MappedBoard = init(board);
    let half = board.keyboard.half;

    let _neopixel_power = Output::new(board.neopixel_power, Level::High);

    //let mut neopixel = Ws2812::new(board.PIO0, Irqs, board.DMA_CH0, board.neopixel);
    //let neopixels_d5 = Ws2812::new(board.PIO1, Irqs, board.DMA_CH1, board.d5);

    let Some([events1, events2]) = board.keyboard.create(spawners).await else {
        log::error!("failed to create keyboard");
        return;
    };

    uart::start(board.tx, board.rx, board.UART0, half, spawners, events2).await;

    // TODO: delaying the logger until here is not ideal
    let usb_logger = usb::driver::setup_logger_and_keyboard(board.USB, events1).await;

    let logger = LogMultiplexer {
        outputs: [rtt_logger, usb_logger],
    };
    logger.init();

    log::error!("log_level: error");
    log::warn!("log_level: warn");
    log::info!("log_level: info");
    log::debug!("log_level: debug");
    log::trace!("log_level: trace");

    board.neopixel.write(&[Rgb::BLUE]).await;

    // fade the led after a few seconds
    Timer::after_secs(5).await;
    for b in (0u8..0xff).rev() {
        board.neopixel.write(&[Rgb::new(0, 0, b)]).await;
        Timer::after_millis(10).await;
    }

    stall().await
}
