use crate::peripherals::PeripheralsNetwork;
use core::slice::from_raw_parts;
use cyw43_pio::PioSpi;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::PIO0,
    pio::{InterruptHandler, Pio},
};

mod server;
mod wireless;

const SERVER_THREADS: usize = 4;
const WIFI_NETWORK: &str = "RI-WLAN";
const WIFI_PASSWORD: &str = "automatica";

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub async fn init(spawner: &Spawner, p: PeripheralsNetwork) {
    info!("Starting network");

    let fw = unsafe {
        from_raw_parts(
            0x10108000 as *const u8,
            include_bytes!("../../../firmware/43439A0.bin").len(),
        )
    };
    let clm = unsafe {
        from_raw_parts(
            0x10148000 as *const u8,
            include_bytes!("../../../firmware/43439A0_clm.bin").len(),
        )
    };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    if let Ok((mut control, stack)) = wireless::init(spawner, pwr, spi, fw, clm).await {
        control.gpio_set(0, true).await;
        server::init(spawner, stack).await;
    }
}
