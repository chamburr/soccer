use crate::{hardware::MOTOR_SIGNAL, peripherals::PeripheralsMotor};
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{
    peripherals::{PWM_CH3, PWM_CH4, PWM_CH5, PWM_CH7},
    pwm::{Channel, Config, Pwm, PwmPinA, PwmPinB},
    Peripheral,
};

struct Motor<T: Channel>(Pwm<'static, T>);

impl<T: Channel> Motor<T> {
    fn new(
        inner: impl Peripheral<P = T> + 'static,
        a: impl Peripheral<P = impl PwmPinA<T>> + 'static,
        b: impl Peripheral<P = impl PwmPinB<T>> + 'static,
    ) -> Self {
        let mut config = Config::default();

        config.top = 254;
        config.divider = 255.into();

        Self(Pwm::new_output_ab(inner, a, b, config))
    }

    fn set_speed(&mut self, speed: i16) {
        let mut config = Config::default();

        config.top = 254;
        config.divider = 255.into();

        if speed >= 0 {
            config.compare_a = speed as u16;
        } else {
            config.compare_b = (-speed) as u16;
        }

        self.0.set_config(&config);
    }
}

#[embassy_executor::task]
async fn motor_task(
    mut motor_fl: Motor<PWM_CH7>,
    mut motor_fr: Motor<PWM_CH3>,
    mut motor_bl: Motor<PWM_CH5>,
    mut motor_br: Motor<PWM_CH4>,
) {
    loop {
        let data = MOTOR_SIGNAL.wait().await; // ISSUE: may hog cpu

        motor_fl.set_speed(-data.fl);
        motor_fr.set_speed(-data.fr);
        motor_bl.set_speed(-data.bl);
        motor_br.set_speed(-data.br);

        // test
        // info!("running motors");
        // motor_fl.set_speed(20);
        // motor_fr.set_speed(20);
        // motor_bl.set_speed(20);
        // motor_br.set_speed(20);
    }
}

pub async fn init(spawner: &Spawner, p: PeripheralsMotor) {
    info!("Starting motor");

    // front right = 6, 7
    // back right = 8, 9
    // back left = 10, 11
    // front left = 14, 15

    let motor_fl = Motor::new(p.PWM_CH7, p.PIN_14, p.PIN_15);
    let motor_fr = Motor::new(p.PWM_CH3, p.PIN_6, p.PIN_7);
    let motor_bl = Motor::new(p.PWM_CH5, p.PIN_10, p.PIN_11);
    let motor_br = Motor::new(p.PWM_CH4, p.PIN_8, p.PIN_9);

    spawner.must_spawn(motor_task(motor_fl, motor_fr, motor_bl, motor_br));
}
