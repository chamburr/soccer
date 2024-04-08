use crate::uart::{Command, UART_CHANNEL};
use embassy_executor::Spawner;
use embassy_futures::join::join4;
use embassy_rp::{
    bind_interrupts,
    i2c::{Async, Config, Error, I2c, Instance, InterruptHandler, SclPin, SdaPin},
    interrupt::typelevel::Binding,
    peripherals::{I2C0, I2C1, PIN_2, PIN_3, PIN_4, PIN_5},
    Peripheral,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Ticker};
use log::{info, warn};

static LIDAR_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static LIDAR_FRONT_SIGNAL: Signal<CriticalSectionRawMutex, (u16, u16)> = Signal::new();
static LIDAR_LEFT_SIGNAL: Signal<CriticalSectionRawMutex, (u16, u16)> = Signal::new();
static LIDAR_RIGHT_SIGNAL: Signal<CriticalSectionRawMutex, (u16, u16)> = Signal::new();
static LIDAR_BACK_SIGNAL: Signal<CriticalSectionRawMutex, (u16, u16)> = Signal::new();

bind_interrupts!(struct Irqs0 {
    I2C0_IRQ => InterruptHandler<I2C0>;
});

bind_interrupts!(struct Irqs1 {
    I2C1_IRQ => InterruptHandler<I2C1>;
});

struct Lidar<T: Instance + 'static>(I2c<'static, T, Async>);

impl<T: Instance> Lidar<T> {
    fn new(
        i2c: impl Peripheral<P = T> + 'static,
        scl: impl Peripheral<P = impl SclPin<T>> + 'static,
        sda: impl Peripheral<P = impl SdaPin<T>> + 'static,
        irqs: impl Binding<T::Interrupt, InterruptHandler<T>>,
    ) -> Self {
        let mut config = Config::default();
        config.frequency = 400000;

        Self(I2c::new_async(i2c, scl, sda, irqs, config))
    }

    async fn init(&mut self, addr: u16) -> Result<(), Error> {
        self.0.write_async(addr, [0x26, 200]).await?; // fps
        Ok(())
    }

    async fn read(&mut self, addr: u16) -> Result<(u16, u16), Error> {
        let mut bytes = [0; 4];
        let _ = self.0.write_read_async(addr, [0x00], &mut bytes).await;

        let dist = u16::from_le_bytes([bytes[0], bytes[1]]);
        let signal = u16::from_le_bytes([bytes[2], bytes[3]]);

        Ok((dist, signal))
    }
}

#[embassy_executor::task]
async fn lidar0_task(mut lidar: Lidar<I2C0>) {
    let mut ticker = Ticker::every(Duration::from_millis(5));

    if lidar.init(0x10).await.is_err() || lidar.init(0x13).await.is_err() {
        warn!("Error initialising lidar 0");
        return;
    }

    loop {
        LIDAR_SIGNAL.signal(());

        if let Ok((dist, signal)) = lidar.read(0x10).await {
            LIDAR_FRONT_SIGNAL.signal((dist, signal));
        } else {
            warn!("Error reading from lidar front");
        }

        if let Ok((dist, signal)) = lidar.read(0x13).await {
            LIDAR_RIGHT_SIGNAL.signal((dist, signal));
        } else {
            warn!("Error reading from lidar right");
        }

        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn lidar1_task(mut lidar: Lidar<I2C1>) {
    if lidar.init(0x12).await.is_err() || lidar.init(0x11).await.is_err() {
        warn!("Error initialising lidar 1");
        return;
    }

    loop {
        LIDAR_SIGNAL.wait().await;

        if let Ok((dist, signal)) = lidar.read(0x12).await {
            LIDAR_LEFT_SIGNAL.signal((dist, signal));
        } else {
            warn!("Error reading from lidar left");
        }

        if let Ok((dist, signal)) = lidar.read(0x11).await {
            LIDAR_BACK_SIGNAL.signal((dist, signal));
        } else {
            warn!("Error reading from lidar back");
        }
    }
}

#[embassy_executor::task]
async fn lidar_task() {
    loop {
        let mut data = join4(
            LIDAR_FRONT_SIGNAL.wait(),
            LIDAR_LEFT_SIGNAL.wait(),
            LIDAR_RIGHT_SIGNAL.wait(),
            LIDAR_BACK_SIGNAL.wait(),
        )
        .await;

        if let Some(new_data) = LIDAR_FRONT_SIGNAL.try_take() {
            data.0 = new_data;
        }

        if let Some(new_data) = LIDAR_LEFT_SIGNAL.try_take() {
            data.1 = new_data;
        }

        if let Some(new_data) = LIDAR_RIGHT_SIGNAL.try_take() {
            data.2 = new_data;
        }

        if let Some(new_data) = LIDAR_BACK_SIGNAL.try_take() {
            data.3 = new_data;
        }

        let _ = UART_CHANNEL.try_send(Command::Lidar {
            front: data.0,
            left: data.1,
            right: data.2,
            back: data.3,
        });
    }
}

pub async fn init(
    spawner: &Spawner,
    i2c0: I2C0,
    scl0: PIN_5,
    sda0: PIN_4,
    i2c1: I2C1,
    scl1: PIN_3,
    sda1: PIN_2,
) {
    info!("Starting lidar");

    let lidar0 = Lidar::new(i2c0, scl0, sda0, Irqs0);
    let lidar1 = Lidar::new(i2c1, scl1, sda1, Irqs1);

    spawner.must_spawn(lidar_task());
    spawner.must_spawn(lidar0_task(lidar0));
    spawner.must_spawn(lidar1_task(lidar1));
}
