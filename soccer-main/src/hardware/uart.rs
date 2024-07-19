use crate::{
    hardware::{ImuData, LidarData, IMU_SIGNAL, LIDAR_SIGNAL},
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
async fn uart_rx_task(mut rx: UartRx<'static, UART1, Async>) {
    let mut buf = [0; 32];
    info!("Started RX task");

    for _ in 0..10 {
        let _ = with_timeout(Duration::from_millis(10), rx.read_to_break(&mut buf)).await;
        info!("Reading data");
    }

    loop {
        // info!("uart loop buf");
        match with_timeout(Duration::from_millis(10), rx.read_to_break(&mut buf)).await {
            Ok(Ok(len)) => match buf[0] {
                1 => {
                    // info!("uart case 1");
                    if len != 17 {
                        warn!("Incorrect length, expected 17, got {}", len);
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

                    // info!("front dist: {}, sig: {}", dis_f, sig_f);
                    // info!("left dist: {}, sig: {}", dis_l, sig_l);
                    // info!("right dist: {}, sig: {}", dis_r, sig_r);
                    // info!("back dist: {}, sig: {}", dis_b, sig_b);
                }
                2 => {
                    // info!("uart case 2");

                    if len != 3 {
                        continue;
                    }

                    let angle = (i16::from_le_bytes([buf[1], buf[2]]) as f32) / 128.;
                    
                    let angle = clamp_angle(angle);

                    IMU_SIGNAL.signal(ImuData {
                        angle: clamp_angle(angle),
                    });

                    // info!("angle: {}", angle);
                }
                _ => {
                    if len != 0 {
                        warn!("Received bad uart data ({}): {}", len, buf);
                    } else {
                        info!("no data");
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

#[embassy_executor::task]
async fn uart_tx_task(mut tx: UartTx<'static, UART1, Async>) {
    let _ = tx.write(&[2, 1, 2, 3, 4]).await;
    let _ = tx.send_break(0).await;
    info!("Started TX task");

    Timer::after_millis(500).await; // wait for reset

    loop {
        // let _ = tx.write(&[2, 1, 2, 3, 4]).await;
        // let _ = tx.send_break(0).await;
        // info!("tx loop");

        // Timer::after_micros(250).await;
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsUart) {
    info!("Starting uart");
    info!("testing");

    let mut config = Config::default();
    config.baudrate = 921600;

    let uart = Uart::new(
        p.UART1, p.PIN_20, p.PIN_21, Irqs, p.DMA_CH3, p.DMA_CH4, config,
    );
    let (tx, rx) = uart.split();

    spawner.must_spawn(uart_rx_task(rx));
    // spawner.must_spawn(uart_tx_task(tx));
}
