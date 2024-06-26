#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type, type_alias_impl_trait)]
#![allow(static_mut_refs)]

use core::panic::PanicInfo;
use cortex_m_rt::ExceptionFrame;
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    config::Config,
    multicore::{spawn_core1, Stack},
    peripherals::CORE1,
    Peripherals,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use log::info;
use static_cell::StaticCell;

mod bluetooth;
mod calibration;
mod fusion;
mod led;
mod lidar;
mod logger;
mod uart;

static CORE_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

static mut STACK1: Stack<65536> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn core0_task(spawner: Spawner, p: Peripherals) {
    logger::init(&spawner, p.USB).await;

    info!("Starting up core 0");

    led::init(&spawner, p.PIO0, p.DMA_CH0, p.PIN_16).await;
    lidar::init(&spawner, p.I2C0, p.PIN_5, p.PIN_4, p.I2C1, p.PIN_3, p.PIN_2).await;
    uart::init(&spawner, p.UART0, p.PIN_0, p.PIN_1, p.DMA_CH1, p.DMA_CH2).await;

    CORE_SIGNAL.wait().await;
    info!("Starting up core 1");
}

#[embassy_executor::task]
async fn core1_task(spawner: Spawner) {
    CORE_SIGNAL.signal(());

    fusion::init(&spawner).await;
}

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Starting up");

    let p = embassy_rp::init(Config::default());

    spawn_core1(
        unsafe { CORE1::steal() },
        unsafe { &mut STACK1 },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| spawner.spawn(core1_task(spawner)).unwrap())
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| spawner.spawn(core0_task(spawner, p)).unwrap())
}

#[cortex_m_rt::exception]
unsafe fn HardFault(_: &ExceptionFrame) -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    cortex_m::asm::udf();
}
