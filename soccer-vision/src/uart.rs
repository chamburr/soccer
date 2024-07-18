use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::{DMA_CH1, DMA_CH2, PIN_0, PIN_1, UART0},
    uart::{Async, Config, InterruptHandler, Uart, UartTx},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;
use num_traits::Float;
use log::{info, warn};

bind_interrupts!(struct Irqs {
    UART0_IRQ => InterruptHandler<UART0>;
});

pub static UART_CHANNEL: Channel<CriticalSectionRawMutex, Command, 6> = Channel::new();

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

                // info!("front dist: {:?}, sig: {:?}", dis_f, sig_f);
                // info!("left dist: {:?}, sig: {:?}", dis_l, sig_l);
                // info!("right dist: {:?}, sig: {:?}", dis_r, sig_r);
                // info!("back dist: {:?}, sig: {:?}", dis_b, sig_b);

                let _ = tx
                    .write(&[
                        1, dis_f[0], dis_f[1], sig_f[0], sig_f[1], dis_l[0], dis_l[1], sig_l[0],
                        sig_l[1], dis_r[0], dis_r[1], sig_r[0], sig_r[1], dis_b[0], dis_b[1],
                        sig_b[0], sig_b[1],
                    ])
                    .await;
                let _ = tx.send_break(0).await;

                // info!("Sent lidar data");
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
    let (tx, _) = uart.split();

    spawner.must_spawn(uart_tx_task(tx));
}
