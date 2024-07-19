use crate::config::set_config;
use crate::peripherals::PeripheralsModule;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};

async fn wait_for(input: &mut Input<'static>, high: bool) {
    if high {
        input.wait_for_low().await;
    } else {
        input.wait_for_high().await;
    }
}

#[embassy_executor::task]
async fn module_task(
    mut pin: Input<'static>,
) {
    info!("Started superteam module task");
    let mut pin_high = pin.is_high();

    loop {
        wait_for(&mut pin, pin_high).await;
        pin_high = !pin_high;
        // MODULE_SIGNAL.signal(pin_high);
        if pin_high {
            set_config!(started, true);
        } else {
            set_config!(started, false);
        }
        // info!("{}", );
        info!("module {}", pin_high);
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsModule) {
    info!("Starting module");
    let pin = Input::new(p.PIN_22, Pull::None);
    spawner.must_spawn(module_task(pin));
}
