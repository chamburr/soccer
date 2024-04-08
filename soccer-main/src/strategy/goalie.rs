use crate::{
    modules::HEADING_SIGNAL,
    strategy::{Data, COORDINATE_SIGNAL, FIELD_LENGTH, FIELD_MARGIN, FIELD_MARGIN_Y, FIELD_WIDTH},
    utils::{clamp_angle, construct_vector},
};
use embassy_time::Instant;
use num_traits::{clamp, Float};

const GOALIE_DISTANCE: f32 = 3.;
const MOVEMENT_THRESHOLD_X: f32 = 7.5;
const MOVEMENT_THRESHOLD_Y: f32 = 7.5;
const CHANGED_THRESHOLD: u64 = 2500;
const MIN_X: f32 = 10.;

pub struct GoalieState {
    pub last_bx: f32,
    pub last_by: f32,
    pub last_changed: Instant,
    pub pushing: bool,
}

impl Default for GoalieState {
    fn default() -> Self {
        Self {
            last_bx: -999.,
            last_by: -999.,
            last_changed: Instant::now(),
            pushing: false,
        }
    }
}

pub async fn run(data: Data, state: &mut GoalieState) {
    let (bx, by, _bok) = data.ball;
    let (x, y, ok) = data.coordinates;

    HEADING_SIGNAL.signal(0.);

    if !ok {
        COORDINATE_SIGNAL.signal((x, y + 5.));
        return;
    }

    if (bx - state.last_bx).abs() > MOVEMENT_THRESHOLD_X
        || (by - state.last_by).abs() > MOVEMENT_THRESHOLD_Y
    {
        state.last_bx = bx;
        state.last_by = by;
        state.last_changed = Instant::now();
    }

    if state.last_changed.elapsed().as_millis() > CHANGED_THRESHOLD {
        state.pushing = true;
    }

    let mut new_x = bx;
    let new_y = FIELD_LENGTH - FIELD_MARGIN_Y - GOALIE_DISTANCE;

    if ok {
        let (_, angle_l) = construct_vector(
            FIELD_WIDTH / 2. - 30. - bx,
            by - FIELD_LENGTH - FIELD_MARGIN,
        );
        let (_, angle_r) = construct_vector(
            FIELD_WIDTH / 2. + 30. - bx,
            by - FIELD_LENGTH - FIELD_MARGIN,
        );
        let mut angle = clamp_angle((angle_l.to_degrees() + angle_r.to_degrees()) / 2.);

        if angle < 0. {
            angle = -180. - angle;
        } else {
            angle = 180. - angle;
        }

        let (sin, cos) = angle.to_radians().sin_cos();
        let mag = (new_y - by) / cos;

        new_x = clamp(
            bx + mag * sin,
            FIELD_MARGIN + MIN_X,
            FIELD_WIDTH - FIELD_MARGIN - MIN_X,
        );
    }

    COORDINATE_SIGNAL.signal((new_x, new_y));
}
