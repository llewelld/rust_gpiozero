//! Blinks an LED : on_time: 2 seconds and off_time: 3 seconds

use palette::rgb::Rgb;
use rust_gpiozero::*;
use std::{thread, time};

fn main() {
    // Create a new LED attached to Pin 17
    let mut led = RGBLED::new(12, 19, 13);

    // Display some colours for half a second each
    led.set_color(Rgb::new(1.0, 1.0, 0.0));
    thread::sleep(time::Duration::from_millis(500));
    led.set_color(Rgb::new(0.0, 1.0, 0.0));
    thread::sleep(time::Duration::from_millis(500));
    led.set_color(Rgb::new(0.0, 1.0, 1.0));
    thread::sleep(time::Duration::from_millis(500));

    // Blink five times
    led.set_blink_count(5);
    led.blink(0.5, 0.2, Rgb::new(1.0, 1.0, 0.0), Rgb::new(0.0, 0.0, 1.0));
    led.wait();
    led.off();

    // Stay off for a second
    thread::sleep(time::Duration::from_millis(1000));

    // Blink ten times
    led.set_blink_count(10);
    led.blink(0.2, 0.5, Rgb::new(1.0, 1.0, 1.0), Rgb::new(1.0, 0.0, 0.0));
    led.wait();

    // Show white for two seconds
    led.on();
    led.set_color(Rgb::new(1.0, 1.0, 1.0));
    thread::sleep(time::Duration::from_millis(2000));
    led.off();
}
