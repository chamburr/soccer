use crate::{
    hardware::{LineData, BALL_SIGNAL, LINE_SIGNAL},
    peripherals::PeripheralsTemts,
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, select4, Either, Either4};
use embassy_rp::gpio::{Input, Pull};

async fn wait_for(input: &mut Input<'static>, high: bool) {
    if high {
        input.wait_for_low().await;
    } else {
        input.wait_for_high().await;
    }
}

#[embassy_executor::task]
async fn temts_task(
    mut line_front: Input<'static>,
    mut line_left: Input<'static>,
    mut line_right: Input<'static>,
    mut line_back: Input<'static>,
    mut ball: Input<'static>,
) {
    info!("Started TEMTs task");
    let mut line_front_high = line_front.is_high();
    let mut line_left_high = line_left.is_high();
    let mut line_right_high = line_right.is_high();
    let mut line_back_high = line_back.is_high();

    let mut ball_high = ball.is_high();

    loop {
        match select(
            select4(
                wait_for(&mut line_front, line_front_high),
                wait_for(&mut line_left, line_left_high),
                wait_for(&mut line_right, line_right_high),
                wait_for(&mut line_back, line_back_high),
            ),
            wait_for(&mut ball, ball_high),
        )
        .await
        {
            Either::First(data) => {
                match data {
                    Either4::First(_) => line_front_high = !line_front_high,
                    Either4::Second(_) => line_left_high = !line_left_high,
                    Either4::Third(_) => line_right_high = !line_right_high,
                    Either4::Fourth(_) => line_back_high = !line_back_high,
                }
                LINE_SIGNAL.signal(LineData {
                    front: line_front_high,
                    left: line_left_high,
                    right: line_right_high,
                    back: line_back_high,
                });
            }
            Either::Second(_) => {
                ball_high = !ball_high;
                // BALL_SIGNAL.signal(ball_high);
            }
        }
        info!("line_front: {}, line_left: {}, line_right: {}, line_back: {}, ball: {}", line_front_high, line_left_high, line_right_high, line_back_high, ball_high);

    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsTemts) {
    info!("Starting temts");

    let line_front = Input::new(p.PIN_1, Pull::None);
    let line_right = Input::new(p.PIN_0, Pull::None);
    let line_left = Input::new(p.PIN_3, Pull::None);
    let line_back = Input::new(p.PIN_2, Pull::None);

    let ball = Input::new(p.PIN_4, Pull::None);

    spawner.must_spawn(temts_task(
        line_front, line_left, line_right, line_back, ball,
    ));
}
