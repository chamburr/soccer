use embassy_rp::gpio::AnyPin;
use log::info;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};

use crate::{
    fusion::{TARE_SIGNAL},
};

#[embassy_executor::task]
async fn button_task(
    mut button: Input<'static>,
) {
    info!("Started Button Task");
    loop {
        button.wait_for_falling_edge().await;
        info!("Button Pressed");
        TARE_SIGNAL.signal(true);
    }
}

pub async fn init(spawner: &Spawner, pin: AnyPin) {
    info!("Starting Button");
    let button = Input::new(pin, Pull::Up);
    spawner.must_spawn(button_task(button));
}
