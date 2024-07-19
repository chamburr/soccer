use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

pub mod camera;
pub mod motor;
pub mod temts;
pub mod uart;
pub mod button;
pub mod superteam_module;

pub static CAMERA_SIGNAL: Signal<CriticalSectionRawMutex, CameraData> = Signal::new();
pub static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();
pub static LIDAR_SIGNAL: Signal<CriticalSectionRawMutex, LidarData> = Signal::new();
pub static LINE_SIGNAL: Signal<CriticalSectionRawMutex, LineData> = Signal::new();
pub static BALL_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static MOTOR_SIGNAL: Signal<CriticalSectionRawMutex, MotorData> = Signal::new();
pub static BUTTON1_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static BUTTON2_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static MODULE_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

pub struct LineData {
    pub front: bool,
    pub left: bool,
    pub right: bool,
    pub back: bool,
}

pub struct LidarData {
    pub front: (u16, u16),
    pub left: (u16, u16),
    pub right: (u16, u16),
    pub back: (u16, u16),
}

pub struct ImuData {
    pub angle: f32,
}

pub struct CameraData {
    pub angle: f32,
    pub dist: f32,
    pub goal_angle: f32,
    pub goal_dist: f32,
}

pub struct MotorData {
    pub fl: i16,
    pub fr: i16,
    pub bl: i16,
    pub br: i16,
}
