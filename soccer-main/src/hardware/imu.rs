use crate::{
    config::{get_config, set_config},
    hardware::{Command, UART_CHANNEL},
    peripherals::PeripheralsImu,
    utils::stop,
};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    i2c::{Async, Config, Error, I2c, InterruptHandler, SclPin, SdaPin},
    peripherals::{I2C1, PIN_18, PIN_19},
    Peripheral,
};
use embassy_time::{with_timeout, Duration, Ticker, Timer};

const IMU_ADDRESS: u16 = 0x68;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<I2C1>;
});

struct Imu(I2c<'static, I2C1, Async>, u16);

impl Imu {
    fn new(
        i2c: impl Peripheral<P = I2C1> + 'static,
        scl: impl Peripheral<P = impl SclPin<I2C1>> + 'static,
        sda: impl Peripheral<P = impl SdaPin<I2C1>> + 'static,
        addr: u16,
    ) -> Self {
        let mut config = Config::default();
        config.frequency = 400000;

        Self(I2c::new_async(i2c, scl, sda, Irqs, config), addr)
    }

    // hacking
    async fn steal(addr: u16) -> Self {
        let i2c = unsafe { I2C1::steal() };
        let scl = unsafe { PIN_19::steal() };
        let sda = unsafe { PIN_18::steal() };

        let mut config = Config::default();
        config.frequency = 400000;

        Self(I2c::new_async(i2c, scl, sda, Irqs, config), addr)
    }

    async fn init(&mut self) -> Result<(), Error> {
        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x06, 0b10000000]).await?; // pwr_mgmt_1
        Timer::after_millis(100).await;

        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x06, 0b00000001]).await?; // pwr_mgmt_1
        Timer::after_millis(100).await;

        self.write_mag(0x32, 0b00000001).await?;
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

        self.write_mag(0x31, 0b00001000).await?;
        self.0.write_async(self.1, [0x7f, 0b00110000]).await?; // reg_bank_sel
        self.0.write_async(self.1, [0x03, 0b10001100]).await?; // i2c_slv0_addr
        self.0.write_async(self.1, [0x04, 0b00010001]).await?; // i2c_slv0_reg
        self.0.write_async(self.1, [0x05, 0b10001000]).await?; // i2c_slv0_ctrl

        self.0.write_async(self.1, [0x7f, 0b00000000]).await?; // reg_bank_sel

        Ok(())
    }

    async fn write_mag(&mut self, reg: u8, data: u8) -> Result<(), Error> {
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

    async fn read(&mut self) -> Result<([i16; 3], [i16; 3], [i16; 3]), Error> {
        let mut bytes = [0; 20];

        self.0.write_read_async(self.1, [0x2d], &mut bytes).await?;

        let acc = [
            i16::from_be_bytes([bytes[0], bytes[1]]),
            i16::from_be_bytes([bytes[2], bytes[3]]),
            i16::from_be_bytes([bytes[4], bytes[5]]),
        ];
        let gyr = [
            i16::from_be_bytes([bytes[6], bytes[7]]),
            i16::from_be_bytes([bytes[8], bytes[9]]),
            i16::from_be_bytes([bytes[10], bytes[11]]),
        ];
        let mag = [
            i16::from_le_bytes([bytes[14], bytes[15]]),
            i16::from_le_bytes([bytes[16], bytes[17]]),
            i16::from_le_bytes([bytes[18], bytes[19]]),
        ];

        Ok((acc, gyr, mag))
    }
}

#[embassy_executor::task]
async fn imu_task(mut power: Output<'static>, mut imu: Imu) {
    let result = with_timeout(Duration::from_millis(500), imu.init()).await;

    if result.is_err() || result.unwrap().is_err() {
        warn!("Error initialising imu");
        return;
    }

    let mut timeouts = 0;

    let mut ticker = Ticker::every(Duration::from_micros(2500));

    loop {
        ticker.reset();

        match with_timeout(Duration::from_millis(5), imu.read()).await {
            Ok(Ok(data)) => {
                let _ = UART_CHANNEL.try_send(Command::Imu {
                    acc: (data.0[0], data.0[1], data.0[2]),
                    gyr: (data.1[0], data.1[1], data.1[2]),
                    mag: (data.2[0], data.2[1], data.2[2]),
                });

                if get_config!(print_imu) {
                    info!(
                        "Imu acc: {}, {}, {}",
                        data.0[0] as f32 / 4096.,
                        data.0[1] as f32 / 4096.,
                        data.0[2] as f32 / 4096.
                    );
                    info!(
                        "Imu gyr: {}, {}, {}",
                        data.1[0] as f32 / 32.8,
                        data.1[1] as f32 / 32.8,
                        data.1[2] as f32 / 32.8
                    );
                    info!(
                        "Imu mag: {}, {}, {}",
                        data.2[0] as f32, data.2[1] as f32, data.2[2] as f32
                    );
                }
                timeouts = 0;
            }
            Ok(Err(_)) => {
                warn!("Error reading from imu");
                timeouts += 1;
            }
            Err(_) => {
                warn!("Timed out reading from imu");
                timeouts += 1;
            }
        }

        if timeouts >= 10 {
            warn!("Resetting imu");
            let started = get_config!(started);
            stop().await;
            timeouts = 0;
            power.set_low();
            Timer::after_millis(100).await;
            power.set_high();
            Timer::after_millis(500).await;
            imu = Imu::steal(imu.1).await;
            let _ = with_timeout(Duration::from_millis(500), imu.init()).await;
            set_config!(started, started);
        }

        ticker.next().await;
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsImu) {
    info!("Starting imu");

    let power = Output::new(p.PIN_28, Level::High);
    let imu = Imu::new(p.I2C1, p.PIN_19, p.PIN_18, IMU_ADDRESS);

    Timer::after_millis(500).await;

    spawner.must_spawn(imu_task(power, imu));
}
