use crate::{
    config::{get_config, set_config},
    hardware::{CameraData, CAMERA_SIGNAL},
    peripherals::PeripheralsCamera,
    utils::stop,
};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::UART0,
    uart::{Async, Config, Error, InterruptHandler, ReadToBreakError, UartRx},
};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal_nb::serial::Read;

bind_interrupts!(struct Irqs {
    UART0_IRQ => InterruptHandler<UART0>;
});

#[embassy_executor::task]
async fn camera_task(mut rx: UartRx<'static, UART0, Async>, mut reset: Output<'static>) {
    info!("camera task started");
    let mut buf = [0; 32];

    for _ in 0..10 {
        let _ = with_timeout(Duration::from_millis(50), rx.read_to_break(&mut buf)).await;
    }

    let mut timeouts = 0;

    loop {
        // info!("camera loop");
        match with_timeout(Duration::from_millis(50), rx.read_to_break(&mut buf)).await {
            Ok(Ok(len)) => {
                match buf[0] {
                    1 => {
                        if len != 9 {
                            warn!("Received bad camera data ({}): {}", len, buf);
                            
                            continue;
                        }

                        let angle = (u16::from_le_bytes([buf[1], buf[2]]) as f32) / 128.;
                        let dist = (u16::from_le_bytes([buf[3], buf[4]]) as f32) / 128.;
                        let goal_angle = (u16::from_le_bytes([buf[5], buf[6]]) as f32) / 128.;
                        let goal_dist = (u16::from_le_bytes([buf[7], buf[8]]) as f32) / 128.;

                        info!("Received camera data: angle {}, dist {}", angle, dist);

                        CAMERA_SIGNAL.signal(CameraData {
                            angle,
                            dist,
                            goal_angle,
                            goal_dist,
                        });

                    }
                    _ => {
                        if len != 0 {
                            warn!("Received bad camera data ({}): {}", len, buf);
                        }
                    }
                }
                timeouts = 0;
            }
            Ok(Err(ReadToBreakError::Other(Error::Overrun))) => {
                warn!("Error receiving camera data: overrun");
                while Read::read(&mut rx) != Err(nb::Error::WouldBlock) {}
            }
            Ok(Err(err)) => {
                warn!("Error receiving camera data: {}", err);
                timeouts += 1;
            }
            Err(_) => {
                warn!("Timed out receiving camera data");
                timeouts += 1;
            }
        }

        if timeouts >= 100 {
            warn!("Resetting camera");
            let started = get_config!(started);
            stop().await;
            timeouts = 0;
            reset.set_low();
            Timer::after_millis(50).await;
            reset.set_high();
            Timer::after_millis(2500).await;
            set_config!(started, started);
        }
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsCamera) {
    info!("Starting camera");

    let mut config = Config::default();
    config.baudrate = 115200;

    let rx = UartRx::new(p.UART0, p.PIN_17, Irqs, p.DMA_CH2, config);
    let reset = Output::new(p.PIN_3, Level::High);

    spawner.must_spawn(camera_task(rx, reset));
}
