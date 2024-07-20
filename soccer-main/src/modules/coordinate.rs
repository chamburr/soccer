use crate::{
    constants::{FIELD_LENGTH, FIELD_WIDTH},
    hardware::{LidarData, LIDAR_SIGNAL},
    modules::{COORDINATE_CHANGED, COORDINATE_MUTEX, HEADING_MUTEX, UNIGNORE_SIGNAL},
    utils::{debug::debug_variable, read_mutex, write_mutex},
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use num_traits::Float;

const LIDAR_DIST_MIN: f32 = 20.;
const LIDAR_SIGNAL_MIN: u16 = 200;
const LIDAR_CHANGE_TOLERANCE: f32 = 5.;
const LIDAR_IGNORE_TOLERANCE: i32 = 100;

const FIELD_LENGTH_TOLERANCE: f32 = 14.;
const FIELD_WIDTH_TOLERANCE: f32 = 8.;

#[embassy_executor::task]
async fn coordinate_task() {
    let publisher = COORDINATE_CHANGED.immediate_publisher();

    let mut last_front = 0.;
    let mut last_back = 0.;
    let mut last_left = 0.;
    let mut last_right = 0.;

    let mut ignore_front = 0;
    let mut ignore_back = 0;
    let mut ignore_left = 0;
    let mut ignore_right = 0;

    loop {
        let (left, right, front, back);

        match select(LIDAR_SIGNAL.wait(), UNIGNORE_SIGNAL.wait()).await {
            Either::First(data) => {
                LidarData {
                    left,
                    right,
                    front,
                    back,
                } = data;
            }
            Either::Second(data) => {
                if data.0 && ignore_front != 0 {
                    ignore_front = -1;
                }
                if data.1 && ignore_left != 0 {
                    ignore_left = -1;
                }
                if data.2 && ignore_right != 0 {
                    ignore_right = -1;
                }
                if data.3 && ignore_back != 0 {
                    ignore_back = -1;
                }
                continue;
            }
        }

        let heading = read_mutex!(HEADING_MUTEX);
        let cos = heading.to_radians().cos().abs();

        let mut left = ((left.0 as f32) * cos + 3.5, left.1);
        let mut right = ((right.0 as f32) * cos + 3.5, right.1);
        let mut front = ((front.0 as f32) * cos + 3.5, front.1);
        let mut back = ((back.0 as f32) * cos + 3.5, back.1);

        if heading.abs() <= 45. {
            (front, back, left, right) = (front, back, left, right);
        } else if heading.abs() >= 135. {
            (front, back, left, right) = (back, front, right, left);
        } else if heading > 0. {
            (front, back, left, right) = (right, left, back, front);
        } else if heading < 0. {
            (front, back, left, right) = (left, right, front, back);
        }

        // info!("L {} {} R {} {} F {} {} B {} {}", left.0, left.1, right.0, right.1, front.0, front.1, back.0, back.1);

        debug_variable!("lidar dist left", left.0);
        debug_variable!("lidar dist right", right.0);
        debug_variable!("lidar dist front", front.0);
        debug_variable!("lidar dist back", back.0);

        debug_variable!("lidar ignore left", ignore_left);
        debug_variable!("lidar ignore right", ignore_right);
        debug_variable!("lidar ignore front", ignore_front);
        debug_variable!("lidar ignore back", ignore_back);

        if (front.1).min(back.1) > LIDAR_SIGNAL_MIN
            && (front.0 + back.0 - FIELD_LENGTH).abs() < FIELD_LENGTH_TOLERANCE
        {
            ignore_front = 0;
            ignore_back = 0;
        } else {
            if front.0 < LIDAR_DIST_MIN
                || front.1 < LIDAR_SIGNAL_MIN
                || (front.0 - last_front).abs() > LIDAR_CHANGE_TOLERANCE
            {
                ignore_front += 1;
            } else if ignore_front <= LIDAR_IGNORE_TOLERANCE {
                ignore_front = 0;
            }
            if back.0 < LIDAR_DIST_MIN
                || back.1 < LIDAR_SIGNAL_MIN
                || (back.0 - last_back).abs() > LIDAR_CHANGE_TOLERANCE
            {
                ignore_back += 1;
            } else if ignore_back <= LIDAR_IGNORE_TOLERANCE {
                ignore_back = 0;
            }
        }

        if (left.1).min(right.1) > LIDAR_SIGNAL_MIN
            && (left.0 + right.0 - FIELD_WIDTH).abs() < FIELD_WIDTH_TOLERANCE
        {
            ignore_left = 0;
            ignore_right = 0;
        } else {
            if left.0 < LIDAR_DIST_MIN
                || left.1 < LIDAR_SIGNAL_MIN
                || (left.0 - last_left).abs() > LIDAR_CHANGE_TOLERANCE
            {
                ignore_left += 1;
            } else if ignore_left <= LIDAR_IGNORE_TOLERANCE {
                ignore_left = 0;
            }
            if right.0 < LIDAR_DIST_MIN
                || right.1 < LIDAR_SIGNAL_MIN
                || (right.0 - last_right).abs() > LIDAR_CHANGE_TOLERANCE
            {
                ignore_right += 1;
            } else if ignore_right <= LIDAR_IGNORE_TOLERANCE {
                ignore_right = 0;
            }
        }

        if (ignore_front != 0 && ignore_back != 0) || (ignore_left != 0 && ignore_right != 0) {
            debug_variable!("lidar ok", false);
            let (x, y, _) = read_mutex!(COORDINATE_MUTEX);
            write_mutex!(COORDINATE_MUTEX, (x, y, false));
            publisher.publish_immediate(());
            continue;
        }

        let use_front = if ignore_front <= 0 && ignore_back <= 0 {
            front.1 > back.1
        } else {
            ignore_front <= 0
        };

        let use_left = if ignore_left <= 0 && ignore_right <= 0 {
            left.1 > right.1
        } else {
            ignore_left <= 0
        };

        let x = if use_left {
            left.0
        } else {
            FIELD_WIDTH - right.0
        };

        let y = if use_front {
            front.0
        } else {
            FIELD_LENGTH - back.0
        };

        if ignore_front <= 0 {
            last_front = front.0;
        }
        if ignore_back <= 0 {
            last_back = back.0;
        }
        if ignore_left <= 0 {
            last_left = left.0;
        }
        if ignore_right <= 0 {
            last_right = right.0;
        }

        // info!("L {}, R {}, F {}, B {}", ignore_left, ignore_right, ignore_front, ignore_back);


        write_mutex!(COORDINATE_MUTEX, (x, y, true));
        publisher.publish_immediate(());

        debug_variable!("lidar ok", true);
        debug_variable!("lidar x", x);
        debug_variable!("lidar y", y);
    }
}

pub async fn init(spawner: &Spawner) {
    info!("Starting coordinate");

    spawner.must_spawn(coordinate_task());
}
