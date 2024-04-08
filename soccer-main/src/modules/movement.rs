use crate::{
    config::get_config,
    constants::{FIELD_LENGTH, FIELD_MARGIN, FIELD_MARGIN_X, FIELD_MARGIN_Y, FIELD_WIDTH},
    hardware::{MotorData, MOTOR_SIGNAL},
    modules::{
        COORDINATE_CHANGED, COORDINATE_MUTEX, COORDINATE_SIGNAL, HEADING_CHANGED, HEADING_MUTEX,
        HEADING_SIGNAL,
    },
    utils::{clamp_angle, construct_vector, debug::debug_variable, read_mutex},
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use num_traits::Float;
use pid::Pid;

const MOTOR_MIN: f32 = 26.;
const MOTOR_MIN_FINAL: f32 = 27.;
const MOTOR_ANGLE_RATIO: f32 = 0.2;
const MOTOR_POSITION_RATIO: f32 = 0.8;
const NO_COORDINATE_MAX: f32 = 0.5;

const STRIKER_DISTANCE: f32 = 30.;

pub static SPEED_ANGLE_SIGNAL: Signal<CriticalSectionRawMutex, (f32, f32)> = Signal::new();
pub static ROTATION_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

pub fn drive(speed: f32, angle: f32, rotation: f32) {
    let angle = clamp_angle(45. - angle).to_radians();
    let speed = speed * MOTOR_POSITION_RATIO * (255. - MOTOR_MIN);

    let (sin, cos) = angle.sin_cos();
    let max = sin.abs().max(cos.abs());

    let speed_x = speed * cos / max;
    let speed_y = speed * sin / max;

    let mut speed_fl = speed_y;
    let mut speed_fr = -speed_x;
    let mut speed_bl = speed_x;
    let mut speed_br = -speed_y;

    speed_fl += rotation * MOTOR_ANGLE_RATIO * (255. - MOTOR_MIN);
    speed_fr += rotation * MOTOR_ANGLE_RATIO * (255. - MOTOR_MIN);
    speed_bl += rotation * MOTOR_ANGLE_RATIO * (255. - MOTOR_MIN);
    speed_br += rotation * MOTOR_ANGLE_RATIO * (255. - MOTOR_MIN);

    speed_fl += speed_fl.signum() * MOTOR_MIN;
    speed_fr += speed_fr.signum() * MOTOR_MIN;
    speed_bl += speed_bl.signum() * MOTOR_MIN;
    speed_br += speed_br.signum() * MOTOR_MIN;

    if speed_fl.abs() < MOTOR_MIN_FINAL {
        speed_fl = 0.;
    }

    if speed_fr.abs() < MOTOR_MIN_FINAL {
        speed_fr = 0.;
    }

    if speed_bl.abs() < MOTOR_MIN_FINAL {
        speed_bl = 0.;
    }

    if speed_br.abs() < MOTOR_MIN_FINAL {
        speed_br = 0.;
    }

    MOTOR_SIGNAL.signal(MotorData {
        fl: speed_fl.round() as i16,
        fr: speed_fr.round() as i16,
        bl: speed_bl.round() as i16,
        br: speed_br.round() as i16,
    });
}

#[embassy_executor::task]
async fn speed_angle_task() {
    let mut target = COORDINATE_SIGNAL.wait().await;
    let mut subscriber = COORDINATE_CHANGED.subscriber().unwrap();

    loop {
        if !get_config!(started) {
            target = COORDINATE_SIGNAL.wait().await;
            if !get_config!(started) {
                continue;
            }
        }

        let mut pid = Pid::new(0., 1.);
        pid.p(get_config!(pid2_p), 1.).d(get_config!(pid2_d), 1.);

        loop {
            match select(COORDINATE_SIGNAL.wait(), subscriber.next_message()).await {
                Either::First(data) => {
                    if target != data || !get_config!(started) {
                        target = data;
                        break;
                    }
                }
                Either::Second(_) => {
                    let heading = read_mutex!(HEADING_MUTEX);
                    let (x, y, ok) = read_mutex!(COORDINATE_MUTEX);

                    let goalie = get_config!(goalie);

                    let (tx, ty) = if ok {
                        if target.0 > FIELD_MARGIN_X && target.0 < FIELD_WIDTH - FIELD_MARGIN_X {
                            let goalie_y = if !goalie {
                                FIELD_LENGTH - FIELD_MARGIN_Y - STRIKER_DISTANCE
                            } else {
                                FIELD_LENGTH - FIELD_MARGIN_Y
                            };
                            (
                                target.0.clamp(FIELD_MARGIN, FIELD_WIDTH - FIELD_MARGIN),
                                target.1.clamp(FIELD_MARGIN_Y, goalie_y),
                            )
                        } else {
                            let goalie_y = if !goalie {
                                FIELD_LENGTH - FIELD_MARGIN_Y - STRIKER_DISTANCE
                            } else {
                                FIELD_LENGTH - FIELD_MARGIN
                            };
                            (
                                target.0.clamp(FIELD_MARGIN, FIELD_WIDTH - FIELD_MARGIN),
                                target.1.clamp(FIELD_MARGIN, goalie_y),
                            )
                        }
                    } else {
                        (target.0, target.1)
                    };

                    debug_variable!("target x", tx);
                    debug_variable!("target y", ty);

                    let (x_diff, y_diff) = (tx - x, y - ty);
                    let (distance, angle) = construct_vector(x_diff, y_diff);
                    let angle = clamp_angle(angle.to_degrees() - heading);
                    let mut speed = -pid.next_control_output(distance).output;

                    if !ok {
                        speed = speed.min(NO_COORDINATE_MAX);
                    }

                    SPEED_ANGLE_SIGNAL.signal((speed, angle));
                    debug_variable!("pid speed", speed);
                    debug_variable!("pid angle", angle);
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn rotation_task() {
    let mut target = HEADING_SIGNAL.wait().await;
    let mut subscriber = HEADING_CHANGED.subscriber().unwrap();

    loop {
        if !get_config!(started) {
            target = HEADING_SIGNAL.wait().await;
            if !get_config!(started) {
                continue;
            }
        }

        let mut pid = Pid::new(0., 1.);
        pid.p(get_config!(pid_p), 1.).d(get_config!(pid_d), 1.);

        loop {
            match select(HEADING_SIGNAL.wait(), subscriber.next_message()).await {
                Either::First(data) => {
                    if target != data || !get_config!(started) {
                        target = data;
                        break;
                    }
                }
                Either::Second(_) => {
                    let heading = read_mutex!(HEADING_MUTEX);
                    let angle = clamp_angle(heading - target);
                    let rotation = pid.next_control_output(angle).output;
                    ROTATION_SIGNAL.signal(rotation);
                    debug_variable!("pid rotation", rotation);
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn drive_task() {
    let (mut speed, mut angle) = SPEED_ANGLE_SIGNAL.wait().await;
    let mut rotation = ROTATION_SIGNAL.wait().await;

    loop {
        match select(SPEED_ANGLE_SIGNAL.wait(), ROTATION_SIGNAL.wait()).await {
            Either::First(data) => (speed, angle) = data,
            Either::Second(data) => rotation = data,
        }

        drive(speed, angle, rotation);
    }
}

pub async fn init(spawner: &Spawner) {
    info!("Starting movement");

    spawner.must_spawn(rotation_task());
    spawner.must_spawn(speed_angle_task());
    spawner.must_spawn(drive_task());
}
