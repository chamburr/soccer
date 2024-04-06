use crate::network::{SERVER_THREADS, WIFI_NETWORK, WIFI_PASSWORD};
use cyw43::{Control, NetDriver, PowerManagementMode::PowerSave, Runner, State};
use cyw43_pio::PioSpi;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
use embassy_net_driver::{Driver, HardwareAddress};
use embassy_net_driver_channel::Device;
use embassy_rp::{
    clocks::RoscRng,
    gpio::Output,
    peripherals::{DMA_CH0, PIO0},
};
use embassy_time::Timer;
use heapless::Vec;
use rand::Rng;
use static_cell::make_static;

#[embassy_executor::task]
async fn wifi_task(runner: Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<NetDriver<'static>>) {
    stack.run().await
}

pub async fn init(
    spawner: &Spawner,
    pwr: Output<'static>,
    spi: PioSpi<'static, PIO0, 0, DMA_CH0>,
    fw: &[u8],
    clm: &[u8],
) -> Result<(Control<'static>, &'static Stack<Device<'static, 1514>>), ()> {
    let state = make_static!(State::new());
    let (device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.must_spawn(wifi_task(runner));

    control.init(clm).await;
    control.set_power_management(PowerSave).await;

    info!("Joining Wifi network: {}", WIFI_NETWORK);

    for i in 0..3 {
        if let Err(err) = if WIFI_PASSWORD.is_empty() {
            control.join_open(WIFI_NETWORK).await
        } else {
            control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await
        } {
            warn!("Failed to join Wifi network: {}", err.status);
            if i != 2 {
                Timer::after_millis(100).await;
                continue;
            } else {
                return Err(());
            }
        }
        break;
    }

    info!("Connected to Wifi network");

    let mut ip = Ipv4Address::new(192, 168, 1, 69);
    if let HardwareAddress::Ethernet(addr) = device.hardware_address() {
        ip.0[3] = 50 + addr[5] % 50;
    }

    info!("Using IP address: {}", ip);

    let stack = make_static!(Stack::new(
        device,
        Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(ip, 24),
            gateway: None,
            dns_servers: Vec::new(),
        }),
        make_static!(StackResources::<SERVER_THREADS>::new()),
        RoscRng.gen(),
    ));

    spawner.must_spawn(net_task(stack));

    Ok((control, stack))
}
