use crate::{
    config::get_config,
    hardware::{Command, ImuData, LidarData, IMU_SIGNAL, LIDAR_SIGNAL, UART_CHANNEL},
    peripherals::PeripheralsUart,
    utils::clamp_angle,
};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::UART1,
    uart::{Async, Config, Error, InterruptHandler, ReadToBreakError, Uart, UartRx, UartTx},
};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal_nb::serial::Read;

bind_interrupts!(struct Irqs {
    UART1_IRQ => InterruptHandler<UART1>;
});

#[embassy_executor::task]
async fn uart_tx_task(mut tx: UartTx<'static, UART1, Async>) {
    let _ = tx.write(&[2, 1, 2, 3, 4]).await;
    let _ = tx.send_break(0).await;

    Timer::after_millis(500).await; // wait for reset

    for _ in 0..4 {
        let _ = UART_CHANNEL.try_receive();
    }

    loop {
        match UART_CHANNEL.receive().await {
            Command::Imu { acc, gyr, mag } => {
                let acc_x = acc.0.to_le_bytes();
                let acc_y = acc.1.to_le_bytes();
                let acc_z = acc.2.to_le_bytes();
                let gyr_x = gyr.0.to_le_bytes();
                let gyr_y = gyr.1.to_le_bytes();
                let gyr_z = gyr.2.to_le_bytes();
                let mag_x = mag.0.to_le_bytes();
                let mag_y = mag.1.to_le_bytes();
                let mag_z = mag.2.to_le_bytes();

                let _ = tx
                    .write(&[
                        1, acc_x[0], acc_x[1], acc_y[0], acc_y[1], acc_z[0], acc_z[1], gyr_x[0],
                        gyr_x[1], gyr_y[0], gyr_y[1], gyr_z[0], gyr_z[1], mag_x[0], mag_x[1],
                        mag_y[0], mag_y[1], mag_z[0], mag_z[1],
                    ])
                    .await;
                let _ = tx.send_break(0).await;
            }
        }

        Timer::after_micros(250).await;
    }
}

#[embassy_executor::task]
async fn uart_rx_task(mut rx: UartRx<'static, UART1, Async>) {
    let mut buf = [0; 32];

    Timer::after_millis(500).await; // wait for reset

    for _ in 0..10 {
        let _ = with_timeout(Duration::from_millis(10), rx.read_to_break(&mut buf)).await;
    }

    loop {
        match with_timeout(Duration::from_millis(10), rx.read_to_break(&mut buf)).await {
            Ok(Ok(len)) => match buf[0] {
                1 => {
                    if len != 17 {
                        continue;
                    }

                    let dis_f = u16::from_le_bytes([buf[1], buf[2]]);
                    let sig_f = u16::from_le_bytes([buf[3], buf[4]]);
                    let dis_l = u16::from_le_bytes([buf[5], buf[6]]);
                    let sig_l = u16::from_le_bytes([buf[7], buf[8]]);
                    let dis_r = u16::from_le_bytes([buf[9], buf[10]]);
                    let sig_r = u16::from_le_bytes([buf[11], buf[12]]);
                    let dis_b = u16::from_le_bytes([buf[13], buf[14]]);
                    let sig_b = u16::from_le_bytes([buf[15], buf[16]]);

                    LIDAR_SIGNAL.signal(LidarData {
                        front: (dis_f, sig_f),
                        left: (dis_l, sig_l),
                        right: (dis_r, sig_r),
                        back: (dis_b, sig_b),
                    });
                }
                2 => {
                    if len != 3 {
                        continue;
                    }

                    let mut zero_angle = get_config!(angle);
                    if zero_angle >= 999. {
                        zero_angle = 0.;
                    }

                    let angle = (i16::from_le_bytes([buf[1], buf[2]]) as f32) / 128.;
                    let angle = clamp_angle(angle - zero_angle);

                    IMU_SIGNAL.signal(ImuData {
                        angle: clamp_angle(angle),
                    });
                }
                _ => {
                    if len != 0 {
                        warn!("Received bad uart data ({}): {}", len, buf);
                    }
                }
            },
            Ok(Err(ReadToBreakError::Other(Error::Overrun))) => {
                warn!("Error receiving uart data: overrun");
                while Read::read(&mut rx) != Err(nb::Error::WouldBlock) {}
            }
            Ok(Err(err)) => {
                warn!("Error receiving uart data: {}", err);
            }
            _ => {}
        }
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsUart) {
    info!("Starting uart");

    let mut config = Config::default();
    config.baudrate = 921600;

    let uart = Uart::new(
        p.UART1, p.PIN_20, p.PIN_21, Irqs, p.DMA_CH3, p.DMA_CH4, config,
    );
    let (tx, rx) = uart.split();

    spawner.must_spawn(uart_tx_task(tx));
    spawner.must_spawn(uart_rx_task(rx));
}
