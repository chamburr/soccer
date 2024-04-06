use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, pubsub::PubSubChannel,
    signal::Signal,
};

pub mod ball;
pub mod coordinate;
pub mod heading;
pub mod movement;

// current
pub static HEADING_MUTEX: Mutex<CriticalSectionRawMutex, f32> = Mutex::new(0.);
pub static COORDINATE_MUTEX: Mutex<CriticalSectionRawMutex, (f32, f32, bool)> =
    Mutex::new((0., 0., false));
pub static BALL_MUTEX: Mutex<CriticalSectionRawMutex, (f32, f32, bool)> =
    Mutex::new((0., 0., false));
pub static GOAL_MUTEX: Mutex<CriticalSectionRawMutex, (f32, f32, bool)> =
    Mutex::new((0., 0., false));

// target
pub static HEADING_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();
pub static COORDINATE_SIGNAL: Signal<CriticalSectionRawMutex, (f32, f32)> = Signal::new();
pub static UNIGNORE_SIGNAL: Signal<CriticalSectionRawMutex, (bool, bool, bool, bool)> =
    Signal::new();

// alerts
type Alert<T> = PubSubChannel<CriticalSectionRawMutex, T, 1, 2, 0>;
pub static HEADING_CHANGED: Alert<()> = PubSubChannel::new();
pub static COORDINATE_CHANGED: Alert<()> = PubSubChannel::new();
pub static BALL_CHANGED: Alert<bool> = PubSubChannel::new();
