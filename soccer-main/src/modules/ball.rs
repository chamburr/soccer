use crate::{
    hardware::CAMERA_SIGNAL,
    modules::{
        BALL_CHANGED, BALL_MUTEX, COORDINATE_CHANGED, COORDINATE_MUTEX, GOAL_MUTEX, HEADING_MUTEX,
    },
    utils::{clamp_angle, debug::debug_variable, read_mutex, write_mutex},
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use nalgebra::{Rotation2, Vector2};

#[embassy_executor::task]
async fn ball_task() {
    let publisher = BALL_CHANGED.immediate_publisher();
    let mut subscriber = COORDINATE_CHANGED.subscriber().unwrap();
    let mut camera = CAMERA_SIGNAL.wait().await;

    #[allow(unused_assignments)]
    let (mut x, mut y, mut ok) = read_mutex!(COORDINATE_MUTEX);

    loop {
        let is_camera = match select(CAMERA_SIGNAL.wait(), subscriber.next_message()).await {
            Either::First(data) => {
                camera = data;
                true
            }
            Either::Second(_) => {
                (x, y, ok) = read_mutex!(COORDINATE_MUTEX);
                if !ok {
                    publisher.publish_immediate(false);
                    continue;
                }
                false
            }
        };

        debug_variable!("camera angle", camera.angle);
        debug_variable!("camera dist", camera.dist);
        debug_variable!("camera goal angle", camera.goal_angle);
        debug_variable!("camera goal dist", camera.goal_dist);

        let heading = read_mutex!(HEADING_MUTEX);

        if camera.angle == 0. && camera.dist == 0. {
            let (bx, by, _) = read_mutex!(BALL_MUTEX);
            write_mutex!(BALL_MUTEX, (bx, by, false));
        } else {
            let angle = clamp_angle(heading + camera.angle);
            let vector = Vector2::new(x, y);
            let rotation = Rotation2::new(angle.to_radians());
            let translation = Vector2::y() * camera.dist;
            let vector = vector - rotation * translation;

            write_mutex!(BALL_MUTEX, (vector.x, vector.y, true));
        }

        if camera.goal_angle == 0. && camera.goal_dist == 0. {
            let (gx, gy, _) = read_mutex!(GOAL_MUTEX);
            write_mutex!(GOAL_MUTEX, (gx, gy, false));
        } else {
            let angle = clamp_angle(heading + camera.goal_angle);
            let vector = Vector2::new(x, y);
            let rotation = Rotation2::new(angle.to_radians());
            let translation = Vector2::y() * camera.goal_dist;
            let vector = vector - rotation * translation;

            write_mutex!(GOAL_MUTEX, (vector.x, vector.y, true));
        }

        publisher.publish_immediate(is_camera);
    }
}

pub async fn init(spawner: &Spawner) {
    info!("Starting ball");

    spawner.must_spawn(ball_task());
}
