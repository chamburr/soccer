use crate::{
    config::get_config,
    constants::{
        BALLCAP_DISTANCE, BALLCAP_WIDTH, CLEARANCE_Y, FIELD_LENGTH, FIELD_MARGIN, FIELD_MARGIN_X,
        FIELD_MARGIN_Y, FIELD_WIDTH,
    },
    hardware::{BALL_SIGNAL, LINE_SIGNAL},
    modules::{
        BALL_CHANGED, BALL_MUTEX, COORDINATE_MUTEX, COORDINATE_SIGNAL, HEADING_SIGNAL,
        UNIGNORE_SIGNAL,
    },
    strategy::{
        attack::AttackState, bounds::BoundsState, clear::ClearState, defence::DefenceState,
        get_out::GetOutState, goalie::GoalieState, no_ball::NoBallState,
    },
    utils::{construct_vector, debug::debug_variable, read_mutex},
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_sync::pubsub::WaitResult;
use embassy_time::Instant;
use num_traits::{clamp, Float};

pub mod attack;
pub mod bounds;
pub mod clear;
pub mod defence;
pub mod get_out;
pub mod goalie;
pub mod no_ball;

const STRATEGY_DURATION: u64 = 15;
const BOUNDS_DURATION: u64 = 100;
const NO_BALL_DURATION: u64 = 500;
const GOALIE_NO_BALL_DURATION: u64 = 200; // goalie should return home more
const GOALIE_ATTACK_DURATION: u64 = 6000;
const STRIKER_DISTANCE: f32 = 30.;

#[derive(Clone, Copy, PartialEq)]
pub enum Strategy {
    None,
    Attack,
    Bounds,
    Clear,
    Defence,
    Goalie,
    GetOut,
    NoBall,
}

#[derive(Default)]
pub struct Data {
    pub ball: (f32, f32, bool),
    pub coordinates: (f32, f32, bool),
    pub captured: bool,
    pub lines: (bool, bool, bool, bool),
    pub goalie: bool,
    pub is_camera: bool,
}

#[embassy_executor::task]
async fn strategy_task() {
    let mut subscriber = BALL_CHANGED.subscriber().unwrap();

    let mut ball = read_mutex!(BALL_MUTEX);
    let mut coordinates = read_mutex!(COORDINATE_MUTEX);
    let mut captured = false;
    let mut lines = (false, false, false, false);
    let mut is_camera = false;

    let mut last_strategy = Strategy::None;
    let mut last_changed = Instant::now();
    let mut last_ball_found = Instant::now();
    let mut last_goalie_attacked = Instant::from_millis(0);

    let mut state_attack = AttackState::default();
    let mut state_bounds = BoundsState::default();
    let mut state_clear = ClearState::default();
    let mut state_defence = DefenceState::default();
    let mut state_get_out = GetOutState::default();
    let mut state_goalie = GoalieState::default();
    let mut state_no_ball = NoBallState::default();

    loop {
        match select3(
            LINE_SIGNAL.wait(),
            subscriber.next_message(),
            BALL_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(data) => {
                lines = (data.front, data.left, data.right, data.back);
                UNIGNORE_SIGNAL.signal(lines);
                is_camera = false;
            }
            Either3::Second(data) => {
                ball = read_mutex!(BALL_MUTEX);
                coordinates = read_mutex!(COORDINATE_MUTEX);
                if let WaitResult::Message(data2) = data {
                    is_camera = data2;
                }
            }
            Either3::Third(data) => {
                captured = data;
                is_camera = false;
            }
        }

        let goalie = get_config!(goalie);
        let data = Data {
            ball,
            coordinates,
            captured,
            lines,
            goalie,
            is_camera,
        };

        let mut strategy;
        let (x, y, ok) = data.coordinates;
        let (mut bx, mut by, bok) = data.ball;

        if ok {
            bx = clamp(bx, 0., FIELD_WIDTH);
            by = clamp(by, 0., FIELD_LENGTH);
        }

        debug_variable!("ball x", bx);
        debug_variable!("ball y", by);

        if bok {
            last_ball_found = Instant::now();
        }

        let (dist, _) = construct_vector(x - bx, y - by);

        let no_ball_duration = if !goalie {
            NO_BALL_DURATION
        } else {
            GOALIE_NO_BALL_DURATION
        };

        let striker_distance = if !goalie { STRIKER_DISTANCE } else { 0. };

        if ok
            && dist < 50.
            && last_ball_found.elapsed().as_millis() < no_ball_duration
            && !(FIELD_MARGIN_X..=FIELD_WIDTH - FIELD_MARGIN_X).contains(&bx)
            && by < FIELD_MARGIN + 10.
            && by < y
        {
            strategy = Strategy::Clear;
        } else if ok && !((FIELD_MARGIN_Y - 5.)..=FIELD_LENGTH - FIELD_MARGIN_Y + 5.).contains(&y) {
            strategy = Strategy::GetOut;
        } else if last_ball_found.elapsed().as_millis() > no_ball_duration {
            strategy = Strategy::NoBall;
        } else if dist > 50. {
            strategy = Strategy::Attack;
        } else if ok
            && (by > y || (by + BALLCAP_DISTANCE > y && (x - bx).abs() > BALLCAP_WIDTH / 2.))
            && by > FIELD_LENGTH - FIELD_MARGIN_Y - CLEARANCE_Y - striker_distance
        {
            strategy = Strategy::Defence;
        } else {
            strategy = Strategy::Attack;
        }

        if goalie && (strategy == Strategy::Clear || strategy == Strategy::Attack) {
            if last_strategy == Strategy::Goalie && state_goalie.pushing {
                last_goalie_attacked = Instant::now();
            }
            if last_goalie_attacked.elapsed().as_millis() < GOALIE_ATTACK_DURATION
                && strategy == Strategy::Attack
            {
                strategy = Strategy::Attack;
            } else {
                strategy = Strategy::Goalie;
            }
        }

        if lines.0 || lines.1 || lines.2 || lines.3 {
            strategy = Strategy::Bounds;
            last_changed = Instant::now();
        } else if strategy != last_strategy
            && ((last_strategy == Strategy::Bounds
                && last_changed.elapsed().as_millis() < BOUNDS_DURATION)
                || (last_strategy != Strategy::Bounds
                    && last_changed.elapsed().as_millis() < STRATEGY_DURATION))
        {
            strategy = last_strategy;
        } else {
            last_changed = Instant::now();
        }

        // if get_config!(go_home) {
        //     HEADING_SIGNAL.signal(0.);
        //     COORDINATE_SIGNAL.signal((FIELD_WIDTH / 2., FIELD_LENGTH - FIELD_MARGIN_Y));
        //     continue;
        // }

        // if get_config!(go_other) {
        //     HEADING_SIGNAL.signal(0.);
        //     COORDINATE_SIGNAL.signal((FIELD_MARGIN, FIELD_MARGIN_Y));
        //     continue;
        // }

        match strategy {
            Strategy::Attack => {
                if strategy != last_strategy {
                    state_attack = AttackState::default();
                }
                attack::run(data, &mut state_attack).await;
                debug_variable!("strategy", "attack");
            }
            Strategy::Bounds => {
                if strategy != last_strategy {
                    state_bounds = BoundsState::default();
                }
                bounds::run(data, &mut state_bounds).await;
                debug_variable!("strategy", "bounds");
            }
            Strategy::Clear => {
                if strategy != last_strategy {
                    state_clear = ClearState::default();
                }
                clear::run(data, &mut state_clear).await;
                debug_variable!("strategy", "clear");
            }
            Strategy::Defence => {
                if strategy != last_strategy {
                    state_defence = DefenceState::default();
                }
                defence::run(data, &mut state_defence).await;
                debug_variable!("strategy", "defence");
            }
            Strategy::GetOut => {
                if strategy != last_strategy {
                    state_defence = DefenceState::default();
                }
                get_out::run(data, &mut state_get_out).await;
                debug_variable!("strategy", "get_out");
            }
            Strategy::Goalie => {
                if strategy != last_strategy {
                    state_goalie = GoalieState::default();
                }
                goalie::run(data, &mut state_goalie).await;
                debug_variable!("strategy", "goalie");
            }
            Strategy::NoBall => {
                if strategy != last_strategy {
                    state_no_ball = NoBallState::default();
                }
                no_ball::run(data, &mut state_no_ball).await;
                debug_variable!("strategy", "no_ball");
            }
            _ => {}
        }

        last_strategy = strategy;
    }
}

pub async fn init(spawner: &Spawner) {
    info!("Starting strategy");

    spawner.must_spawn(strategy_task());
}
