use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts, clocks,
    dma::{AnyChannel, Channel},
    into_ref,
    peripherals::{DMA_CH0, PIN_16, PIO0},
    pio::{
        Config, FifoJoin, InterruptHandler, Pio, PioPin, ShiftConfig, ShiftDirection, StateMachine,
    },
    Peripheral, PeripheralRef,
};
use embassy_time::Timer;
use fixed::types::U24F8;
use fixed_macro::fixed;
use log::info;
use smart_leds::RGB8;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

struct Led<'d> {
    dma: PeripheralRef<'d, AnyChannel>,
    sm: StateMachine<'d, PIO0, 0>,
}

impl<'d> Led<'d> {
    fn new(pio0: PIO0, dma: impl Peripheral<P = impl Channel> + 'd, pin: impl PioPin) -> Self {
        into_ref!(dma);

        let Pio {
            common: mut pio,
            sm0: mut sm,
            ..
        } = Pio::new(pio0, Irqs);

        let side_set = pio::SideSet::new(false, 1, false);
        let mut a: pio::Assembler<32> = pio::Assembler::new_with_side_set(side_set);

        const T1: u8 = 2;
        const T2: u8 = 5;
        const T3: u8 = 3;
        const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32;

        let mut wrap_target = a.label();
        let mut wrap_source = a.label();
        let mut do_zero = a.label();
        a.set_with_side_set(pio::SetDestination::PINDIRS, 1, 0);
        a.bind(&mut wrap_target);
        a.out_with_delay_and_side_set(pio::OutDestination::X, 1, T3 - 1, 0);
        a.jmp_with_delay_and_side_set(pio::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);
        a.jmp_with_delay_and_side_set(pio::JmpCondition::Always, &mut wrap_target, T2 - 1, 1);
        a.bind(&mut do_zero);
        a.nop_with_delay_and_side_set(T2 - 1, 0);
        a.bind(&mut wrap_source);

        let prg = a.assemble_with_wrap(wrap_source, wrap_target);
        let mut cfg = Config::default();

        let out_pin = pio.make_pio_pin(pin);
        cfg.set_out_pins(&[&out_pin]);
        cfg.set_set_pins(&[&out_pin]);

        cfg.use_program(&pio.load_program(&prg), &[&out_pin]);

        let clock_freq = U24F8::from_num(clocks::clk_sys_freq() / 1000);
        let ws2812_freq = fixed!(800: U24F8);
        let bit_freq = ws2812_freq * CYCLES_PER_BIT;
        cfg.clock_divider = clock_freq / bit_freq;

        cfg.fifo_join = FifoJoin::TxOnly;
        cfg.shift_out = ShiftConfig {
            auto_fill: true,
            threshold: 24,
            direction: ShiftDirection::Left,
        };

        sm.set_config(&cfg);
        sm.set_enable(true);

        Self {
            dma: dma.map_into(),
            sm,
        }
    }

    async fn write(&mut self, color: RGB8) {
        let brightness = 50; // lower is brighter
        let word = (u32::from(color.g / brightness) << 24)
            | (u32::from(color.r / brightness) << 16)
            | (u32::from(color.b / brightness) << 8);

        self.sm.tx().dma_push(self.dma.reborrow(), &[word]).await;
    }
}

fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;

    if wheel_pos < 85 {
        (255 - wheel_pos * 3, 0, wheel_pos * 3).into()
    } else if wheel_pos < 170 {
        wheel_pos -= 85;
        (0, wheel_pos * 3, 255 - wheel_pos * 3).into()
    } else {
        wheel_pos -= 170;
        (wheel_pos * 3, 255 - wheel_pos * 3, 0).into()
    }
}

#[embassy_executor::task]
async fn led_task(mut led: Led<'static>) {
    loop {
        for j in 0..(256 * 5) {
            led.write(wheel(((j as u16) & 255) as u8)).await;
            Timer::after_millis(25).await;
        }
    }
}

pub async fn init(spawner: &Spawner, pio: PIO0, dma: DMA_CH0, pin: PIN_16) {
    info!("Starting led");

    let led = Led::new(pio, dma, pin);

    spawner.spawn(led_task(led)).unwrap();
}
