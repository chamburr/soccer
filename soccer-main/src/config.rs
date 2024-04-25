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
    started: bool = true,
    goalie: bool = true,
    pid_p: f32 = 0.04,
    pid_d: f32 = 0.13,
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
