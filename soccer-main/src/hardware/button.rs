use embassy_rp::gpio::AnyPin;
use defmt::info;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};

use crate::{
    hardware::{BUTTON1_SIGNAL, BUTTON2_SIGNAL},
};
use crate::config::{set_config};

#[embassy_executor::task]
async fn button1_task(
    mut button: Input<'static>,
) {
    info!("Started Button 1 Task");
    loop {
        button.wait_for_falling_edge().await;
        info!("Button 1 Pressed");
        BUTTON1_SIGNAL.signal(true);
        // set_config!(started, true)
    }
}

#[embassy_executor::task]
async fn button2_task(
    mut button: Input<'static>,
) {
    info!("Started Button 1 Task");
    loop {
        button.wait_for_falling_edge().await;
        info!("Button 2 Pressed");
        BUTTON2_SIGNAL.signal(true);
    }
}

pub async fn init(spawner: &Spawner, pin1: AnyPin, pin2:AnyPin) {
    info!("Starting Buttons");
    let button1 = Input::new(pin1, Pull::Up);
    let button2 = Input::new(pin2, Pull::Up);
    spawner.must_spawn(button1_task(button1));
    spawner.must_spawn(button2_task(button2));
}
