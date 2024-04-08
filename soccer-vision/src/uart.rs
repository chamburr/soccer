use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::{DMA_CH1, DMA_CH2, PIN_0, PIN_1, UART0},
    uart::{Async, Config, Error, InterruptHandler, ReadToBreakError, Uart, UartRx, UartTx},
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal_nb::serial::Read;
use log::warn;
use num_traits::Float;

bind_interrupts!(struct Irqs {
    UART0_IRQ => InterruptHandler<UART0>;
});

pub static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();
pub static UART_CHANNEL: Channel<CriticalSectionRawMutex, Command, 6> = Channel::new();

pub struct ImuData {
    pub acc: (f32, f32, f32),
    pub gyr: (f32, f32, f32),
    pub mag: (f32, f32, f32),
}

pub enum Command {
    Lidar {
        front: (u16, u16),
        left: (u16, u16),
        right: (u16, u16),
        back: (u16, u16),
    },
    Positioning {
        angle: f32,
    },
}

#[embassy_executor::task]
async fn uart_tx_task(mut tx: UartTx<'static, UART0, Async>) {
    loop {
        match UART_CHANNEL.receive().await {
            Command::Lidar {
                front,
                left,
                right,
                back,
            } => {
                let dis_f = front.0.to_le_bytes();
                let sig_f = front.1.to_le_bytes();
                let dis_l = left.0.to_le_bytes();
                let sig_l = left.1.to_le_bytes();
                let dis_r = right.0.to_le_bytes();
                let sig_r = right.1.to_le_bytes();
                let dis_b = back.0.to_le_bytes();
                let sig_b = back.1.to_le_bytes();

                let _ = tx
                    .write(&[
                        1, dis_f[0], dis_f[1], sig_f[0], sig_f[1], dis_l[0], dis_l[1], sig_l[0],
                        sig_l[1], dis_r[0], dis_r[1], sig_r[0], sig_r[1], dis_b[0], dis_b[1],
                        sig_b[0], sig_b[1],
                    ])
                    .await;
                let _ = tx.send_break(0).await;
            }
            Command::Positioning { angle } => {
                let angle = ((angle * 128.).round() as i16).to_le_bytes();

                let _ = tx.write(&[2, angle[0], angle[1]]).await;
                let _ = tx.send_break(0).await;
            }
        }

        Timer::after_micros(250).await;
    }
}

#[embassy_executor::task]
async fn uart_rx_task(mut rx: UartRx<'static, UART0, Async>) {
    let mut buf = [0; 32];

    for _ in 0..10 {
        let _ = rx.read_to_break(&mut buf).await;
    }

    loop {
        match with_timeout(Duration::from_millis(10), rx.read_to_break(&mut buf)).await {
            Ok(Ok(len)) => match buf[0] {
                1 => {
                    if len != 19 {
                        continue;
                    }

                    let acc_x = (i16::from_le_bytes([buf[1], buf[2]]) as f32) / 4096.;
                    let acc_y = (i16::from_le_bytes([buf[3], buf[4]]) as f32) / 4096.;
                    let acc_z = (i16::from_le_bytes([buf[5], buf[6]]) as f32) / 4096.;
                    let gyr_x = (i16::from_le_bytes([buf[7], buf[8]]) as f32) / 32.8;
                    let gyr_y = (i16::from_le_bytes([buf[9], buf[10]]) as f32) / 32.8;
                    let gyr_z = (i16::from_le_bytes([buf[11], buf[12]]) as f32) / 32.8;
                    let mag_x = i16::from_le_bytes([buf[13], buf[14]]) as f32;
                    let mag_y = i16::from_le_bytes([buf[15], buf[16]]) as f32;
                    let mag_z = i16::from_le_bytes([buf[17], buf[18]]) as f32;

                    IMU_SIGNAL.signal(ImuData {
                        acc: (acc_x, acc_y, acc_z),
                        gyr: (gyr_x, gyr_y, gyr_z),
                        mag: (mag_x, mag_y, mag_z),
                    });
                }
                2 => {
                    if len != 5 || buf[1] != 1 || buf[2] != 2 || buf[3] != 3 || buf[4] != 4 {
                        continue;
                    }

                    cortex_m::peripheral::SCB::sys_reset();
                }
                _ => {
                    if len != 0 {
                        warn!("Received bad uart data ({}): {:?}", len, buf);
                    }
                }
            },
            Ok(Err(ReadToBreakError::Other(Error::Overrun))) => {
                warn!("Error receiving uart data: overrun");
                while Read::read(&mut rx) != Err(nb::Error::WouldBlock) {}
            }
            Ok(Err(err)) => {
                warn!("Error receiving uart data: {:?}", err);
            }
            _ => {}
        }
    }
}

pub async fn init(
    spawner: &Spawner,
    uart: UART0,
    tx_pin: PIN_0,
    rx_pin: PIN_1,
    tx_dma: DMA_CH1,
    rx_dma: DMA_CH2,
) {
    let mut config = Config::default();
    config.baudrate = 921600;

    let uart = Uart::new(uart, tx_pin, rx_pin, Irqs, tx_dma, rx_dma, config);
    let (tx, rx) = uart.split();

    spawner.must_spawn(uart_tx_task(tx));
    spawner.must_spawn(uart_rx_task(rx));
}
