use embassy_time::Instant;
use crate::{
    fusion::{ImuData, IMU_SIGNAL},
    uart::{Command, UART_CHANNEL},
};
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
use embassy_time::{with_timeout, Duration, Ticker, Timer};
use log::{info, warn};

const IMU_ADDRESS: u16 = 0x68;

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

struct LidarImu<T: Instance + 'static>(I2c<'static, T, Async>, u16);

impl<T: Instance> LidarImu<T> {
    fn new(
        i2c: impl Peripheral<P = T> + 'static,
        scl: impl Peripheral<P = impl SclPin<T>> + 'static,
        sda: impl Peripheral<P = impl SdaPin<T>> + 'static,
        irqs: impl Binding<T::Interrupt, InterruptHandler<T>>,
        imu_addr: u16,
    ) -> Self {
        let mut config = Config::default();
        config.frequency = 400000;

        Self(I2c::new_async(i2c, scl, sda, irqs, config), imu_addr)
    }

    async fn steal0(addr: u16) -> LidarImu<I2C0> {
        let i2c = unsafe { I2C0::steal() };
        let scl = unsafe { PIN_5::steal() };
        let sda = unsafe { PIN_4::steal() };

        let mut config = Config::default();
        config.frequency = 400000;

        LidarImu(I2c::new_async(i2c, scl, sda, Irqs0, config), addr)
    }

    async fn init_lidar(&mut self, addr: u16) -> Result<(), Error> {
        self.0.write_async(addr, [0x26, 200]).await?; // fps
        Ok(())
    }

    async fn read_lidar(&mut self, addr: u16) -> Result<(u16, u16), Error> {
        let mut bytes = [0; 4];
        let _ = self.0.write_read_async(addr, [0x00], &mut bytes).await;

        let dist = u16::from_le_bytes([bytes[0], bytes[1]]);
        let signal = u16::from_le_bytes([bytes[2], bytes[3]]);

        Ok((dist, signal))
    }

    async fn init_imu(&mut self) -> Result<(), Error> {
        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x06, 0b10000000]).await?; // pwr_mgmt_1
        Timer::after_millis(100).await;

        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x06, 0b00000001]).await?; // pwr_mgmt_1
        Timer::after_millis(100).await;

        self.write_imu_mag(0x32, 0b00000001).await?;
        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x03, 0b00000010]).await?; // user_ctrl
        Timer::after_millis(100).await;

        self.0.write_async(self.1, [0x7f, 0b00100000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x01, 0b00000101]).await?; // gyro_config_1
        self.0.write_async(self.1, [0x14, 0b00000101]).await?; // accel_config

        self.0.write_async(self.1, [0x7f, 0b00000011]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x01, 0b00001111]).await?; // i2c_mst_ctrl
        self.0.write_async(self.1, [0x03, 0b00001100]).await?; // i2c_slv0_addr
        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x03, 0b00100000]).await?; // user_ctrl

        self.write_imu_mag(0x31, 0b00001000).await?;
        self.0.write_async(self.1, [0x7f, 0b00110000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x03, 0b10001100]).await?; // i2c_slv0_addr
        self.0.write_async(self.1, [0x04, 0b00010001]).await?; // i2c_slv0_reg
        self.0.write_async(self.1, [0x05, 0b10001000]).await?; // i2c_slv0_ctrl

        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel

        Ok(())
    }

    async fn write_imu_mag(&mut self, reg: u8, data: u8) -> Result<(), Error> {
        self.0.write_async(self.1, [0x7F, 0b00110000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x03, 0b00001100]).await?; // i2c_slv0_addr

        Timer::after_millis(10).await;

        self.0.write_async(self.1, [0x04, reg]).await?; // i2c_slv0_reg
        self.0.write_async(self.1, [0x06, data]).await?; // i2c_slv0_do
        self.0.write_async(self.1, [0x05, 0b10000001]).await?; // i2c_slv0_ctrl

        Timer::after_millis(10).await;

        self.0.write_async(self.1, [0x03, 0b10001100]).await?; // i2c_slv0_addr

        Ok(())
    }

    async fn read_imu(&mut self) -> Result<([f32; 3], [f32; 3], [f32; 3]), Error> {
        let mut bytes = [0; 20];

        self.0.write_read_async(self.1, [0x2d], &mut bytes).await?;

        let acc = [
            i16::from_be_bytes([bytes[0], bytes[1]]),
            i16::from_be_bytes([bytes[2], bytes[3]]),
            i16::from_be_bytes([bytes[4], bytes[5]]),
        ]
        .map(|x| (x as f32) / 4096.);
        let gyr = [
            i16::from_be_bytes([bytes[6], bytes[7]]),
            i16::from_be_bytes([bytes[8], bytes[9]]),
            i16::from_be_bytes([bytes[10], bytes[11]]),
        ]
        .map(|x| (x as f32) / 32.8);
        let mag = [
            i16::from_le_bytes([bytes[14], bytes[15]]),
            i16::from_le_bytes([bytes[16], bytes[17]]),
            i16::from_le_bytes([bytes[18], bytes[19]]),
        ]
        .map(|x| x as f32);

        Ok((acc, gyr, mag))
    }
}

#[embassy_executor::task]
async fn lidar_imu0_task(mut lidar_imu: LidarImu<I2C0>) {
    let result = with_timeout(Duration::from_millis(500), lidar_imu.init_lidar(0x10)).await;
    if result.is_err() || result.unwrap().is_err() {
        warn!("Error initialising lidar front");
        return;
    }

    let result = with_timeout(Duration::from_millis(500), lidar_imu.init_lidar(0x13)).await;
    if result.is_err() || result.unwrap().is_err() {
        warn!("Error initialising lidar right");
        return;
    }

    let result = with_timeout(Duration::from_millis(500), lidar_imu.init_imu()).await;
    if result.is_err() || result.unwrap().is_err() {
        warn!("Error initialising imu");
        return;
    }

    let mut lidar = true;
    let mut ticker = Ticker::every(Duration::from_micros(2500));

    loop {
        let instant = Instant::now();
        let mut dead = false;

        if lidar {
            lidar = false;

            LIDAR_SIGNAL.signal(());

            if let Ok(Ok((dist, signal))) =
                with_timeout(Duration::from_millis(1), lidar_imu.read_lidar(0x10)).await
            {
                LIDAR_FRONT_SIGNAL.signal((dist, signal));
            } else {
                warn!("Error reading from lidar front");
                dead = true;
            }

            if let Ok(Ok((dist, signal))) =
                with_timeout(Duration::from_millis(1), lidar_imu.read_lidar(0x13)).await
            {
                LIDAR_RIGHT_SIGNAL.signal((dist, signal));
            } else {
                warn!("Error reading from lidar right");
                dead = true
            }
        } else {
            lidar = true;
        };

        match with_timeout(Duration::from_millis(2), lidar_imu.read_imu()).await {
            Ok(Ok(data)) => {
                IMU_SIGNAL.signal(ImuData {
                    acc: (data.0[0], data.0[1], data.0[2]),
                    gyr: (data.1[0], data.1[1], data.1[2]),
                    mag: (data.2[0], data.2[1], data.2[2]),
                });

                #[cfg(feature = "calibration")]
                {
                    info!("Imu acc: {}, {}, {}", data.0[0], data.0[1], data.0[2]);
                    info!("Imu gyr: {}, {}, {}", data.1[0], data.1[1], data.1[2]);
                    info!("Imu mag: {}, {}, {}", data.2[0], data.2[1], data.2[2]);
                }
            }
            Ok(Err(err)) => {
                warn!("Error reading from imu {:?}", err);
            }
            Err(_) => {
                warn!("Timed out reading from imu");
                dead = true
            }
        }

        info!("{}", instant.elapsed().as_micros());

        if dead {
            lidar_imu = LidarImu::<I2C0>::steal0(IMU_ADDRESS).await;
            Timer::after_millis(100).await;
            let _ = with_timeout(Duration::from_millis(500), lidar_imu.init_lidar(0x10)).await;
            let _ = with_timeout(Duration::from_millis(500), lidar_imu.init_lidar(0x13)).await;
            let _ = with_timeout(Duration::from_millis(500), lidar_imu.init_imu()).await;
            ticker.reset();
        }

        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn lidar_imu1_task(mut lidar_imu: LidarImu<I2C1>) {
    let result = with_timeout(Duration::from_millis(500), lidar_imu.init_lidar(0x12)).await;
    if result.is_err() || result.unwrap().is_err() {
        warn!("Error initialising lidar left");
        return;
    }

    let result = with_timeout(Duration::from_millis(500), lidar_imu.init_lidar(0x11)).await;
    if result.is_err() || result.unwrap().is_err() {
        warn!("Error initialising lidar back");
        return;
    }

    loop {
        LIDAR_SIGNAL.wait().await;

        if let Ok(Ok((dist, signal))) =
            with_timeout(Duration::from_millis(5), lidar_imu.read_lidar(0x12)).await
        {
            LIDAR_LEFT_SIGNAL.signal((dist, signal));
        } else {
            warn!("Error reading from lidar left");
        }

        if let Ok(Ok((dist, signal))) =
            with_timeout(Duration::from_millis(5), lidar_imu.read_lidar(0x11)).await
        {
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
    info!("Starting lidar imu");

    let lidar_imu0 = LidarImu::new(i2c0, scl0, sda0, Irqs0, IMU_ADDRESS);
    let lidar_imu1 = LidarImu::new(i2c1, scl1, sda1, Irqs1, IMU_ADDRESS);

    spawner.must_spawn(lidar_task());
    spawner.must_spawn(lidar_imu0_task(lidar_imu0));
    spawner.must_spawn(lidar_imu1_task(lidar_imu1));
}
