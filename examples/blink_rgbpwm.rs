//! Blinks an LED : on_time: 2 seconds and off_time: 3 seconds

use rust_gpiozero::*;
use std::{io::Read, time::Duration};

fn input() {
    std::io::stdin().read_exact(&mut [0u8]).unwrap();
}

fn sleep(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

fn main() {
    // Create a new RGBLED attached to Pins 16, 20, 21
    let mut led: RGBPWMLED = RGBPWMLED::new(16, 20, 21, false);

    // Set red
    led.set_value((1.0, 0.0, 0.0));
    sleep(1000);
    led.set_value((0.0, 1.0, 0.0));
    sleep(1000);
    led.set_value((0.0, 0.0, 1.0));
    sleep(1000);
    led.set_value((1.0, 1.0, 0.0));
    sleep(1000);
    led.set_value((1.0, 0.0, 1.0));
    sleep(1000);
    led.set_value((0.0, 1.0, 1.0));
    sleep(1000);
    led.set_value((1.0, 1.0, 1.0));
    sleep(1000);
    input();
    led.stop();
    // Pulse
    led.pulse(3., 3., (1., 0., 0.), (0., 1., 0.));

    input();
    led.stop();

    // Blink: TODO check why behaving differently
    led.blink(3., 3., 3.0, 3.0, (1., 0., 0.), (0., 0., 1.));

    input();
    led.stop();
    // Off
    led.off();
}
