use crate::{
    config::set_config,
    modules::{
        movement::{drive, ROTATION_SIGNAL, SPEED_ANGLE_SIGNAL},
        COORDINATE_MUTEX, COORDINATE_SIGNAL, HEADING_SIGNAL,
    },
};
use num_traits::Float;

#[cfg(feature = "network")]
pub mod debug;
#[cfg(feature = "network")]
pub mod functions;
pub mod logger;

#[cfg(not(feature = "network"))]
pub mod debug {
    macro_rules! debug_variable {
        ($name:literal, $value:expr) => {{
            let _ = $value;
        }};
    }
    pub(crate) use debug_variable;
}

macro_rules! read_mutex {
    ($mutex:expr) => {{
        let mutex_lock = $mutex.lock().await;
        *mutex_lock
    }};
}

macro_rules! write_mutex {
    ($mutex:expr, $value:expr) => {{
        let mut mutex_lock = $mutex.lock().await;
        *mutex_lock = $value;
    }};
}

pub(crate) use read_mutex;
pub(crate) use write_mutex;

pub fn clamp_angle(angle: f32) -> f32 {
    let new_angle = (360. + (angle % 360.)) % 360.;

    if new_angle <= 180. {
        new_angle
    } else {
        new_angle - 360.
    }
}

pub fn construct_vector(x: f32, y: f32) -> (f32, f32) {
    let magnitude = (x * x + y * y).sqrt();
    let angle = x.atan2(y);
    (magnitude, angle)
}

pub async fn start() {
    set_config!(started, true);
}

pub async fn stop() {
    set_config!(started, false);
    let (x, y, _) = read_mutex!(COORDINATE_MUTEX);
    for _ in 0..3 {
        HEADING_SIGNAL.signal(0.);
        COORDINATE_SIGNAL.signal((x, y));
        SPEED_ANGLE_SIGNAL.signal((0., 0.));
        ROTATION_SIGNAL.signal(0.);
    }
    drive(0., 0., 0.);
}
