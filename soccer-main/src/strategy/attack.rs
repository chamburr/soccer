use defmt::info;
use crate::{
    constants::{BALLCAP_DISTANCE, BALLCAP_WIDTH, CLEARANCE_X},
    modules::{COORDINATE_SIGNAL, GOAL_MUTEX, HEADING_SIGNAL},
    strategy::{Data, CLEARANCE_Y, FIELD_MARGIN, FIELD_MARGIN_Y, FIELD_WIDTH},
    utils::{construct_vector, debug::debug_variable, read_mutex},
};
use embassy_time::Instant;
use num_traits::Float;

const ALIGNED_THRESHOLD: f32 = 3.;
const CAPTURED_DURATION: u64 = 100;
const MOVING_BACK_DURATION: u64 = 200;
const INITIAL_CHANGE: f32 = 35.;
const GRADUAL_CHANGE: f32 = 250.;
const ALIGNING_DURATION: u64 = 1000;
const ALIGNING_THRESHOLD: u64 = 2000;

pub struct AttackState {
    pub captured: bool,
    pub last_captured: Instant,
    pub moving_back: bool,
    pub last_moving_back: Instant,
    pub aligned: bool,
    pub initial_change: f32,
    pub initial_magnitude: f32,
    pub last_aligning: Instant,
}

impl Default for AttackState {
    fn default() -> Self {
        Self {
            captured: false,
            last_captured: Instant::now(),
            moving_back: false,
            last_moving_back: Instant::from_millis(0),
            aligned: false,
            initial_change: 0.,
            initial_magnitude: 0.,
            last_aligning: Instant::from_millis(0),
        }
    }
}

pub async fn run(data: Data, state: &mut AttackState) {
    let (bx, by, _bok) = data.ball;
    let (x, y, ok) = data.coordinates;
    let captured = data.captured;

    HEADING_SIGNAL.signal(0.);

    // info!("{}", y -by);

    if captured || (y > by && y < by + 11. && (x - bx).abs() < BALLCAP_WIDTH / 2.) {
        state.last_captured = Instant::now();
    }

    if state.last_captured.elapsed().as_millis() < CAPTURED_DURATION {
        // info!("attack reached");

        if !state.aligned {
            if (x - bx).abs() < ALIGNED_THRESHOLD {
                state.aligned = true;
            }
            COORDINATE_SIGNAL.signal((bx, y));
            return;
        }

        let (goal_x, goal_y) = if ok {
            (FIELD_WIDTH / 2., FIELD_MARGIN)
        } else {
            let goal = read_mutex!(GOAL_MUTEX);
            (goal.0, goal.1)
        };

        let (magnitude, angle) = construct_vector(goal_x - x, y - goal_y);
        let (sin, cos) = angle.sin_cos();

        if state.initial_change == 0. {
            state.initial_magnitude = magnitude;
            state.initial_change = cos.max(0.) * INITIAL_CHANGE;
        }

        let change = (state.initial_change
            + (state.initial_magnitude - magnitude).max(0.) / state.initial_magnitude
                * GRADUAL_CHANGE)
            .min(((y - FIELD_MARGIN_Y) / cos).max(0.));

        // info!("attack change, {}", change);

        let new_x = x + change * sin;
        let new_y = y - change * cos;

        COORDINATE_SIGNAL.signal((new_x, new_y));
        return;
    }

    state.aligned = false;
    state.initial_change = 0.;
    state.initial_magnitude = 0.;

    // info!("attack reached");

    let (mut new_x, mut new_y);

    if y < by {
        state.moving_back = true;
    }

    if (state.moving_back || state.last_moving_back.elapsed().as_millis() > MOVING_BACK_DURATION)
        && (y < by + BALLCAP_DISTANCE / 3.
            || ((x - bx).abs() > BALLCAP_WIDTH / 2. + 5.
                && (x - bx).abs() < CLEARANCE_X / 2.
                && y < by + CLEARANCE_Y / 2.))
    {
        // info!("attack case 1");

        // info!("{}", (y < by + BALLCAP_DISTANCE / 3.));
        // info!("{} {} {}",((x - bx).abs() > BALLCAP_WIDTH / 2. + 5.), (x - bx).abs() < CLEARANCE_X / 2., y < by + CLEARANCE_Y / 2.);

        // info!("x {} y {}", x, y);

        new_x = if ok && bx < FIELD_MARGIN + CLEARANCE_X + 10. {
            bx + (CLEARANCE_X / 2. + 5.)
        } else if ok && bx > FIELD_WIDTH - FIELD_MARGIN - CLEARANCE_X - 10. {
            bx - (CLEARANCE_X / 2. + 5.)
        } else if x > bx {
            bx + (CLEARANCE_X / 2. + 5.)
        } else {
            bx - (CLEARANCE_X / 2. + 5.)
        };
        new_y = if (x - bx).abs() > CLEARANCE_X / 2. + 3. {
            by + CLEARANCE_Y / 2. + 10.
        } else {
            y
        };

        state.moving_back = true;
    } else { // if ball is in front

        if state.moving_back {
            state.moving_back = false;
            state.last_moving_back = Instant::now();
        }

        // info!("attack case 2");

        let aligning = state.last_aligning.elapsed().as_millis();

        // info!("{}", aligning);

        new_x = bx;
        new_y = if (aligning > ALIGNING_DURATION && aligning < ALIGNING_THRESHOLD)
            || (x - bx).abs() < BALLCAP_WIDTH / 2. // actual pushing
        {
            (by + BALLCAP_DISTANCE).min(y - 5.)
        } else { // aligning
            if aligning > ALIGNING_THRESHOLD {
                state.last_aligning = Instant::now();
            }
            by + BALLCAP_DISTANCE + 10. // increase me to slow down as it approaches ball
        };
        if (aligning > ALIGNING_DURATION && aligning < ALIGNING_THRESHOLD)
            || (x - bx).abs() < BALLCAP_WIDTH / 2. // actual pushing
        {
            // info!("pushing");
            // new_x = x;
            // new_y = y;
        } else { // aligning
            // info!("alinging");
            
        };



    }

    COORDINATE_SIGNAL.signal((new_x, new_y));
}
