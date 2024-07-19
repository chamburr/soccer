#![no_std]
#![no_main]
#![feature(const_option, impl_trait_in_assoc_type, type_alias_impl_trait)]
#![allow(static_mut_refs)]

use embassy_time::Timer;
use crate::peripherals::{get_peripherals, Peripherals0, Peripherals1};
use core::panic::PanicInfo;
use cortex_m_rt::ExceptionFrame;
use defmt::info;
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    config::Config,
    multicore::{spawn_core1, Stack},
};
use static_cell::StaticCell;

#[cfg(feature = "network")]
mod bootloader;
mod config;
mod constants;
mod hardware;
mod modules;
#[cfg(feature = "network")]
mod network;
mod peripherals;
mod strategy;
mod utils;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

static mut STACK1: Stack<65536> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn core0_task(spawner: Spawner, p: Peripherals0) {
    info!("Starting up core 0");

    #[cfg(feature = "network")]
    {
        bootloader::init(&spawner, p.bootloader).await;
        network::init(&spawner, p.network).await;
        utils::functions::init().await;
    }

    hardware::uart::init(&spawner, p.uart).await;
    hardware::camera::init(&spawner, p.camera).await;
    hardware::motor::init(&spawner, p.motor).await;
    hardware::temts::init(&spawner, p.temts).await;
    hardware::superteam_module::init(&spawner, p.module).await;
    // hardware::button::init(&spawner, p.button.PIN_22.into(), p.button.PIN_5.into()).await;
    info!("Waiting to start");

    let mut button = p.button.BOOTSEL;
    while !config::get_config!(started) {
        if button.is_pressed() {
            utils::start().await;
            info!("Started");
        }
            info!("Waiting to start");
        Timer::after_millis(10).await;
    }
}

#[embassy_executor::task]
async fn core1_task(spawner: Spawner, _p: Peripherals1) {
    info!("Starting up core 1");

    modules::ball::init(&spawner).await;
    modules::coordinate::init(&spawner).await;
    modules::heading::init(&spawner).await;
    modules::movement::init(&spawner).await;

    strategy::init(&spawner).await;
}

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Starting up");

    unsafe {
        const SIO_BASE: u32 = 0xd000_0000;
        const SPINLOCK_PTR: *mut u32 = (SIO_BASE + 0x100) as *mut u32;
        const SPINLOCK_COUNT: usize = 32;
        for i in 0..SPINLOCK_COUNT {
            SPINLOCK_PTR.wrapping_add(i).write_volatile(1);
        }
    }

    let p = embassy_rp::init(Config::default());
    let (core1, p0, p1) = get_peripherals(p);

    spawn_core1(core1, unsafe { &mut STACK1 }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| spawner.spawn(core1_task(spawner, p1)).unwrap())
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| spawner.spawn(core0_task(spawner, p0)).unwrap())
}

#[cortex_m_rt::exception]
unsafe fn HardFault(_: &ExceptionFrame) -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    cortex_m::asm::udf();
}
