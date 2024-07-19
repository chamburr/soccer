use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

macro_rules! init_config {
    ($($name:ident: $type:ty = $value:literal),*,) => {
        pub static CONFIG: Mutex<CriticalSectionRawMutex, Config> = Mutex::new(Config {
            $(
                $name: $value,
            )*
        });

        pub struct Config {
            $(
                pub $name: $type,
            )*
        }
    };
}

init_config! {
    started: bool = false,
    goalie: bool = false,
    // rotation
    pid_p: f32 = 0.2, // .08 is oks
    pid_d: f32 = 0.07,
    // movement
    pid2_p: f32 = 0.10, // .08 is oks
    pid2_d: f32 = 0.1,
}

macro_rules! get_config {
    ($name:ident) => {{
        let config = crate::config::CONFIG.lock().await;
        config.$name.clone()
    }};
}

macro_rules! set_config {
    ($name:ident, $value:expr) => {{
        let mut config = crate::config::CONFIG.lock().await;
        config.$name = $value;
    }};
}

pub(crate) use get_config;
pub(crate) use set_config;
