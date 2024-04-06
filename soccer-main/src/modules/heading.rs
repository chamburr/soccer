use crate::{
    hardware::{ImuData, IMU_SIGNAL},
    modules::{HEADING_CHANGED, HEADING_MUTEX},
    utils::{debug::debug_variable, write_mutex},
};
use defmt::info;
use embassy_executor::Spawner;

#[embassy_executor::task]
async fn heading_task() {
    let publisher = HEADING_CHANGED.immediate_publisher();

    loop {
        let ImuData { angle } = IMU_SIGNAL.wait().await;

        write_mutex!(HEADING_MUTEX, angle);
        publisher.publish_immediate(());

        debug_variable!("heading", angle);
    }
}

pub async fn init(spawner: &Spawner) {
    info!("Starting heading");

    spawner.must_spawn(heading_task());
}
