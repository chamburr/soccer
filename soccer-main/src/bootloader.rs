use crate::peripherals::PeripheralsBootloader;
use defmt::info;
use embassy_boot_rp::{AlignedBuffer, FirmwareUpdater, FirmwareUpdaterConfig, State::Swap};
use embassy_executor::Spawner;
use embassy_rp::{flash::Flash, watchdog::Watchdog};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Channel,
    mutex::Mutex,
};
use embassy_time::{Duration, Timer};
use embedded_storage_async::nor_flash::NorFlash;
use heapless::Vec;

pub enum Command {
    Prepare,
    Commit,
    Restart,
    WriteChunk { offset: u32, buffer: Vec<u8, 128> },
}

pub static BOOTLOADER_CHANNEL: Channel<CriticalSectionRawMutex, Command, 1> = Channel::new();

#[embassy_executor::task]
async fn bootloader_task(p: PeripheralsBootloader) {
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    let flash_mutex: Mutex<NoopRawMutex, _> =
        Mutex::new(Flash::<_, _, 2097152>::new(p.FLASH, p.DMA_CH1));
    let updater_config = FirmwareUpdaterConfig::from_linkerfile(&flash_mutex, &flash_mutex);
    let mut aligned = AlignedBuffer([0; 4]);
    let mut updater = FirmwareUpdater::new(updater_config, &mut aligned.0);

    if updater.get_state().await.unwrap() != Swap {
        Timer::after_millis(500).await; // catch early panics
        info!("Applied update");
        updater.mark_booted().await.unwrap();
    }

    let mut writer: Option<_> = None;
    let mut chunk = AlignedBuffer([0; 128]);

    loop {
        match BOOTLOADER_CHANNEL.receive().await {
            Command::Prepare => {
                info!("Preparing for update");
                writer = Some(updater.prepare_update().await.unwrap());
            }
            Command::Commit => {
                info!("Marking as updated");
                writer = None;
                updater.mark_updated().await.unwrap();
            }
            Command::Restart => {
                info!("Restarting");
                watchdog.start(Duration::from_millis(250));
            }
            Command::WriteChunk { offset, buffer } => {
                if offset % 10000 <= 127 {
                    info!("Writing update chunk {}", offset);
                }
                chunk.0[..buffer.len()].copy_from_slice(buffer.as_slice());
                if let Some(ref mut w) = writer {
                    w.write(offset, &chunk.0[..]).await.unwrap();
                }
            }
        }
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsBootloader) {
    info!("Starting bootloader");

    spawner.must_spawn(bootloader_task(p));
}
