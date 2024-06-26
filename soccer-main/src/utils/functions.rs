use crate::{
    bootloader::{Command, BOOTLOADER_CHANNEL},
    config::set_config,
    modules::{movement, HEADING_SIGNAL},
    utils,
    utils::debug::debug_functions,
};

debug_functions! {
    async fn start() {
        utils::start().await;
    }

    async fn drive(speed: f32, angle: f32, rotation: f32) {
        movement::drive(speed, angle, rotation);
    }

    async fn set_goalie(enable: bool) {
        set_config!(goalie, enable);
    }

    async fn set_pid(p1: f32, d1: f32, p2: f32, d2: f32) {
        set_config!(pid_p, p1);
        set_config!(pid_d, d1);
        set_config!(pid2_p, p2);
        set_config!(pid2_d, d2);
        HEADING_SIGNAL.signal(0.01);
    }

    async fn print_imu(enable: bool) {
        set_config!(print_imu, enable);
    }

    async fn stop() {
        utils::stop().await;
    }

    async fn restart() {
        BOOTLOADER_CHANNEL.send(Command::Restart).await;
    }
}
