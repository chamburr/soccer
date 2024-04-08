use crate::{
    calibration::{ACC_MISALIGNMENT, ACC_OFFSET, HARD_IRON_OFFSET, SOFT_IRON_MATRIX},
    uart::{Command, IMU_SIGNAL, UART_CHANNEL},
};
use embassy_executor::Spawner;
use embassy_time::{Instant, Timer};
use imu_fusion::{FusionAhrs, FusionAhrsSettings, FusionConvention, FusionMatrix, FusionVector};
use num_traits::Float;

const GAIN: f32 = 1.5;
const REJECTION_ACC: f32 = 0.;
const REJECTION_MAG: f32 = 0.;
const REJECTION_PERIOD: i32 = 300;

const MAGNETOMETER_MAX: f32 = 1500.;
const MAGNETOMETER_GYR_MAX: f32 = 30.;

struct Offset {
    coefficient: f32,
    threshold: f32,
    readings: u32,
    timer: u32,
    initialised: bool,
    initialised2: bool,
    offset: FusionVector,
    total: FusionVector,
}

impl Offset {
    fn new() -> Self {
        Self {
            coefficient: 0.05,
            threshold: 1.5,
            readings: 150,
            timer: 0,
            initialised: false,
            initialised2: false,
            offset: FusionVector::zero(),
            total: FusionVector::zero(),
        }
    }

    fn update(&mut self, mut gyr: FusionVector) -> FusionVector {
        gyr -= self.offset;

        if self.timer == 0 {
            self.total = FusionVector::zero();
        }

        if self.initialised && gyr.x.abs().max(gyr.y.abs()).max(gyr.z.abs()) > self.threshold {
            self.timer = 0;
            if !self.initialised2 {
                self.initialised = false;
                self.offset = FusionVector::zero();
            }
            return gyr;
        }

        if self.timer < self.readings {
            self.timer += 1;
            self.total += gyr;
            return gyr;
        }

        self.timer = 0;

        if !self.initialised {
            self.initialised = true;
            self.offset = self.total * (1. / (self.readings as f32));
            return gyr;
        }

        if !self.initialised2 {
            self.initialised2 = true;
            return gyr;
        }

        self.offset += self.total * (self.coefficient / (self.readings as f32));

        gyr
    }
}

fn clamp_angle(angle: f32) -> f32 {
    let new_angle = (360. + (angle % 360.)) % 360.;

    if new_angle <= 180. {
        new_angle
    } else {
        new_angle - 360.
    }
}

#[embassy_executor::task]
async fn fusion_task() {
    let mut offset = Offset::new();
    let mut fusion = FusionAhrs::new();
    let mut must_reset = false;

    fusion.update_settings(FusionAhrsSettings {
        convention: FusionConvention::NWU,
        gain: GAIN,
        gyr_range: 0.,
        acc_rejection: REJECTION_ACC,
        mag_rejection: REJECTION_MAG,
        recovery_trigger_period: REJECTION_PERIOD,
    });

    while !offset.initialised2 {
        let data = IMU_SIGNAL.wait().await;
        offset.update(FusionVector::new(data.gyr.0, data.gyr.1, data.gyr.2));
    }

    let acc_offset = FusionVector::new(ACC_OFFSET[0], ACC_OFFSET[1], ACC_OFFSET[2]);

    let acc_misalignment = FusionMatrix::new(
        ACC_MISALIGNMENT[0][0],
        ACC_MISALIGNMENT[0][1],
        ACC_MISALIGNMENT[0][2],
        ACC_MISALIGNMENT[1][0],
        ACC_MISALIGNMENT[1][1],
        ACC_MISALIGNMENT[1][2],
        ACC_MISALIGNMENT[2][0],
        ACC_MISALIGNMENT[2][1],
        ACC_MISALIGNMENT[2][2],
    );

    let hard_iron_offset = FusionVector::new(
        HARD_IRON_OFFSET[0],
        HARD_IRON_OFFSET[1],
        HARD_IRON_OFFSET[2],
    );

    let soft_iron_matrix = FusionMatrix::new(
        SOFT_IRON_MATRIX[0][0],
        SOFT_IRON_MATRIX[0][1],
        SOFT_IRON_MATRIX[0][2],
        SOFT_IRON_MATRIX[1][0],
        SOFT_IRON_MATRIX[1][1],
        SOFT_IRON_MATRIX[1][2],
        SOFT_IRON_MATRIX[2][0],
        SOFT_IRON_MATRIX[2][1],
        SOFT_IRON_MATRIX[2][2],
    );

    Timer::after_millis(100).await;

    IMU_SIGNAL.wait().await;
    let mut prev_time = Instant::now();

    loop {
        let data = IMU_SIGNAL.wait().await;
        let dt = prev_time.elapsed().as_micros() as f32 / 1000000.;

        if dt >= 0.5 {
            must_reset = true;
        }

        prev_time = Instant::now();

        let acc = FusionVector::new(data.acc.0, data.acc.1, data.acc.2);
        let acc = acc_misalignment * (acc - acc_offset);
        let acc = FusionVector::new(acc.x, acc.y, -acc.z);

        let gyr = FusionVector::new(-data.gyr.0, -data.gyr.1, data.gyr.2);
        let gyr = offset.update(gyr);

        let mag = FusionVector::new(data.mag.0, data.mag.1, data.mag.2);
        let mag = soft_iron_matrix * (mag - hard_iron_offset);
        let mag = FusionVector::new(mag.x, -mag.y, mag.z);

        if mag.x.abs().max(mag.y.abs()).max(mag.z.abs()) > MAGNETOMETER_MAX
            || gyr.x.abs().max(gyr.y.abs()).max(gyr.z.abs()) > MAGNETOMETER_GYR_MAX
        {
            fusion.update(gyr, acc, FusionVector::zero(), dt);
        } else {
            fusion.update(gyr, acc, mag, dt);
        }

        if must_reset {
            must_reset = false;
            fusion.magnetic_recovery_trigger = REJECTION_PERIOD;
            continue;
        }

        let angle = clamp_angle(fusion.quaternion.euler().angle.yaw);
        let _ = UART_CHANNEL.try_send(Command::Positioning { angle });
    }
}

pub async fn init(spawner: &Spawner) {
    spawner.must_spawn(fusion_task());
}
