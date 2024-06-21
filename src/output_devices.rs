//! Output device component interfaces for devices such as `LED`, `PWMLED`, etc
use palette::rgb::Rgb;
use rppal::gpio::{Gpio, IoPin, Level, Mode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

/// Represents a generic GPIO output device.
#[derive(Debug)]
pub struct OutputDevice {
    pin: IoPin,
    active_state: bool,
    inactive_state: bool,
}
#[macro_export]
macro_rules! impl_io_device {
    () => {
        #[allow(dead_code)]
        fn value_to_state(&self, value: bool) -> bool {
            if value {
                self.active_state
            } else {
                self.inactive_state
            }
        }

        fn state_to_value(&self, state: bool) -> bool {
            state == self.active_state
        }

        /// Returns ``True`` if the device is currently active and ``False`` otherwise.
        pub fn value(&self) -> bool {
            match self.pin.read() {
                Level::Low => self.state_to_value(false),
                Level::High => self.state_to_value(true),
            }
        }
    };
}

macro_rules! impl_output_device {
    () => {
        /// Set the state for active_high
        pub fn set_active_high(&mut self, value: bool) {
            if value {
                self.active_state = true;
                self.inactive_state = false;
            } else {
                self.active_state = false;
                self.inactive_state = true;
            }
        }
        /// When ``True``, the `value` property is ``True`` when the device's
        /// `pin` is high. When ``False`` the `value` property is
        /// ``True`` when the device's pin is low (i.e. the value is inverted).
        /// Be warned that changing it will invert `value` (i.e. changing this property doesn't change
        /// the device's pin state - it just changes how that state is interpreted).
        pub fn active_high(&self) -> bool {
            self.active_state
        }

        /// Turns the device on.
        pub fn on(&mut self) {
            self.write_state(true)
        }

        /// Turns the device off.
        pub fn off(&mut self) {
            self.write_state(false)
        }
        /// Reverse the state of the device. If it's on, turn it off; if it's off, turn it on.
        pub fn toggle(&mut self) {
            if self.is_active() {
                self.off()
            } else {
                self.on()
            }
        }
        fn write_state(&mut self, value: bool) {
            if self.value_to_state(value) {
                self.pin.set_high()
            } else {
                self.pin.set_low()
            }
        }
    };
}

impl OutputDevice {
    /// Returns an OutputDevice with the pin number given

    ///
    /// * `pin` - The GPIO pin which the device is attached to
    ///  
    pub fn new(pin: u8) -> OutputDevice {
        match Gpio::new() {
            Err(e) => panic!("{:?}", e),
            Ok(gpio) => match gpio.get(pin) {
                Err(e) => panic!("{:?}", e),
                Ok(pin) => OutputDevice {
                    pin: pin.into_io(Mode::Output),
                    active_state: true,
                    inactive_state: false,
                },
            },
        }
    }

    impl_device!();
    impl_gpio_device!();
    impl_io_device!();
    impl_output_device!();
}

/// Represents a generic output device with typical on/off behaviour.
/// Extends behaviour with a blink() method which uses a background
/// thread to handle toggling the device state without further interaction.
#[derive(Debug)]
pub struct DigitalOutputDevice {
    device: Arc<Mutex<OutputDevice>>,
    blinking: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    blink_count: Option<i32>,
}

macro_rules! impl_digital_output_device {
    () => {
        fn blinker(&mut self, on_time: f32, off_time: f32, n: Option<i32>) {
            self.stop();

            let device = Arc::clone(&self.device);
            let blinking = Arc::clone(&self.blinking);

            self.handle = Some(thread::spawn(move || {
                blinking.store(true, Ordering::SeqCst);
                match n {
                    Some(end) => {
                        for _ in 0..end {
                            if !blinking.load(Ordering::SeqCst) {
                                device.lock().unwrap().off();
                                break;
                            }
                            device.lock().unwrap().on();
                            thread::sleep(Duration::from_millis((on_time * 1000.0) as u64));
                            device.lock().unwrap().off();
                            thread::sleep(Duration::from_millis((off_time * 1000.0) as u64));
                        }
                    }
                    None => loop {
                        if !blinking.load(Ordering::SeqCst) {
                            device.lock().unwrap().off();
                            break;
                        }
                        device.lock().unwrap().on();
                        thread::sleep(Duration::from_millis((on_time * 1000.0) as u64));
                        device.lock().unwrap().off();
                        thread::sleep(Duration::from_millis((off_time * 1000.0) as u64));
                    },
                }
            }));
        }
        /// Returns ``True`` if the device is currently active and ``False`` otherwise.
        pub fn is_active(&self) -> bool {
            Arc::clone(&self.device).lock().unwrap().is_active()
        }
        /// Turns the device on.
        pub fn on(&self) {
            self.stop();
            self.device.lock().unwrap().on()
        }
        /// Turns the device off.
        pub fn off(&self) {
            self.stop();
            self.device.lock().unwrap().off()
        }

        /// Reverse the state of the device. If it's on, turn it off; if it's off, turn it on.
        pub fn toggle(&mut self) {
            self.device.lock().unwrap().toggle()
        }

        /// Returns ``True`` if the device is currently active and ``False`` otherwise.
        pub fn value(&self) -> bool {
            self.device.lock().unwrap().value()
        }

        fn stop(&self) {
            self.blinking.clone().store(false, Ordering::SeqCst);
            self.device.lock().unwrap().pin.set_low();
        }

        /// When ``True``, the `value` property is ``True`` when the device's
        /// `pin` is high. When ``False`` the `value` property is
        /// ``True`` when the device's pin is low (i.e. the value is inverted).
        /// Be warned that changing it will invert `value` (i.e. changing this property doesn't change
        /// the device's pin state - it just changes how that state is interpreted).
        pub fn active_high(&self) -> bool {
            self.device.lock().unwrap().active_high()
        }

        /// Set the state for active_high
        pub fn set_active_high(&mut self, value: bool) {
            self.device.lock().unwrap().set_active_high(value)
        }

        /// The `Pin` that the device is connected to.
        pub fn pin(&self) -> u8 {
            self.device.lock().unwrap().pin.pin()
        }

        /// Shut down the device and release all associated resources.
        pub fn close(self) {
            drop(self)
        }

        /// Block until background process is done
        pub fn wait(&mut self) {
            self.handle
                .take()
                .expect("Called stop on non-running thread")
                .join()
                .expect("Could not join spawned thread");
        }
    };
}

impl DigitalOutputDevice {
    pub fn new(pin: u8) -> DigitalOutputDevice {
        DigitalOutputDevice {
            device: Arc::new(Mutex::new(OutputDevice::new(pin))),
            blinking: Arc::new(AtomicBool::new(false)),
            handle: None,
            blink_count: None,
        }
    }

    impl_digital_output_device!();

    /// Make the device turn on and off repeatedly in the background.
    /// Use `set_blink_count` to set the number of times to blink the device
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    ///
    pub fn blink(&mut self, on_time: f32, off_time: f32) {
        match self.blink_count {
            None => self.blinker(on_time, off_time, None),
            Some(n) => self.blinker(on_time, off_time, Some(n)),
        }
    }
    /// Set the number of times to blink the device
    /// * `n` - Number of times to blink
    pub fn set_blink_count(&mut self, n: i32) {
        self.blink_count = Some(n)
    }
}

pub struct RGBLED {
    red: Arc<Mutex<OutputDevice>>,
    green: Arc<Mutex<OutputDevice>>,
    blue: Arc<Mutex<OutputDevice>>,
    blinking: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    blink_count: Option<i32>,
}

impl RGBLED {
    pub fn new(pin_red: u8, pin_green: u8, pin_blue: u8, active_high: bool) -> RGBLED {
        let red = Arc::new(Mutex::new(OutputDevice::new(pin_red)));
        let green = Arc::new(Mutex::new(OutputDevice::new(pin_green)));
        let blue = Arc::new(Mutex::new(OutputDevice::new(pin_blue)));
        red.lock().unwrap().set_active_high(active_high);
        green.lock().unwrap().set_active_high(active_high);
        blue.lock().unwrap().set_active_high(active_high);
        Self {
            red,
            green,
            blue,
            blinking: Arc::new(AtomicBool::new(false)),
            handle: None,
            blink_count: None,
        }
    }

    pub fn set_color(&mut self, color: Rgb) {
        Self::write_color(&self.red, &self.green, &self.blue, color);
    }

    fn write_color(
        red: &Arc<Mutex<OutputDevice>>,
        green: &Arc<Mutex<OutputDevice>>,
        blue: &Arc<Mutex<OutputDevice>>,
        color: Rgb,
    ) {
        Self::write_state(&red, color.red > 0.5);
        Self::write_state(&green, color.green > 0.5);
        Self::write_state(&blue, color.blue > 0.5);
    }

    fn write_state(device: &Arc<Mutex<OutputDevice>>, value: bool) {
        if device.lock().unwrap().value_to_state(value) {
            device.lock().unwrap().pin.set_high()
        } else {
            device.lock().unwrap().pin.set_low()
        }
    }

    fn blinker(
        &mut self,
        on_time: f32,
        off_time: f32,
        on_color: Rgb,
        off_color: Rgb,
        n: Option<i32>,
    ) {
        self.stop();

        let red = Arc::clone(&self.red);
        let green = Arc::clone(&self.green);
        let blue = Arc::clone(&self.blue);

        let blinking = Arc::clone(&self.blinking);

        self.handle = Some(thread::spawn(move || {
            blinking.store(true, Ordering::SeqCst);
            match n {
                Some(end) => {
                    for _ in 0..end {
                        if !blinking.load(Ordering::SeqCst) {
                            red.lock().unwrap().off();
                            green.lock().unwrap().off();
                            blue.lock().unwrap().off();
                            break;
                        }
                        Self::write_color(&red, &green, &blue, on_color);
                        thread::sleep(Duration::from_millis((on_time * 1000.0) as u64));
                        Self::write_color(&red, &green, &blue, off_color);
                        thread::sleep(Duration::from_millis((off_time * 1000.0) as u64));
                    }
                }
                None => loop {
                    if !blinking.load(Ordering::SeqCst) {
                        red.lock().unwrap().off();
                        green.lock().unwrap().off();
                        blue.lock().unwrap().off();
                        break;
                    }
                    Self::write_color(&red, &green, &blue, on_color);
                    thread::sleep(Duration::from_millis((on_time * 1000.0) as u64));
                    Self::write_color(&red, &green, &blue, off_color);
                    thread::sleep(Duration::from_millis((off_time * 1000.0) as u64));
                },
            }
        }));
    }
    /// Returns ``True`` if the device is currently active and ``False`` otherwise.
    pub fn is_active(&self) -> bool {
        self.red.lock().unwrap().is_active()
            || self.green.lock().unwrap().is_active()
            || self.blue.lock().unwrap().is_active()
    }
    /// Turns the device on.
    pub fn on(&self) {
        self.stop();
        self.red.lock().unwrap().on();
        self.green.lock().unwrap().on();
        self.blue.lock().unwrap().on();
    }
    /// Turns the device off.
    pub fn off(&self) {
        self.stop();
        self.red.lock().unwrap().off();
        self.green.lock().unwrap().off();
        self.blue.lock().unwrap().off();
    }
    /// Reverse the state of the device. If it's on, turn it off; if it's off, turn it on.
    pub fn toggle(&mut self) {
        if self.is_active() {
            self.on()
        } else {
            self.off()
        }
    }

    /// Returns ``True`` if the device is currently active and ``False`` otherwise.
    pub fn value_red(&self) -> bool {
        self.red.lock().unwrap().value()
    }

    /// Returns ``True`` if the device is currently active and ``False`` otherwise.
    pub fn value_green(&self) -> bool {
        self.green.lock().unwrap().value()
    }

    /// Returns ``True`` if the device is currently active and ``False`` otherwise.
    pub fn value_blue(&self) -> bool {
        self.blue.lock().unwrap().value()
    }

    fn stop(&self) {
        self.blinking.clone().store(false, Ordering::SeqCst);
        self.red.lock().unwrap().pin.set_low();
        self.green.lock().unwrap().pin.set_low();
        self.blue.lock().unwrap().pin.set_low();
    }

    /// When ``True``, the `value` property is ``True`` when the device's
    /// `pin` is high. When ``False`` the `value` property is
    /// ``True`` when the device's pin is low (i.e. the value is inverted).
    /// Be warned that changing it will invert `value` (i.e. changing this property doesn't change
    /// the device's pin state - it just changes how that state is interpreted).
    pub fn active_high(&self) -> bool {
        self.red.lock().unwrap().active_high()
            || self.green.lock().unwrap().active_high()
            || self.blue.lock().unwrap().active_high()
    }

    /// Set the state for active_high
    pub fn set_active_high(&mut self, value: bool) {
        self.red.lock().unwrap().set_active_high(value);
        self.green.lock().unwrap().set_active_high(value);
        self.blue.lock().unwrap().set_active_high(value);
    }

    /// The `Pin` that the device is connected to.
    pub fn pin_red(&self) -> u8 {
        self.red.lock().unwrap().pin.pin()
    }

    /// The `Pin` that the device is connected to.
    pub fn pin_green(&self) -> u8 {
        self.green.lock().unwrap().pin.pin()
    }

    /// The `Pin` that the device is connected to.
    pub fn pin_blue(&self) -> u8 {
        self.blue.lock().unwrap().pin.pin()
    }

    /// Shut down the device and release all associated resources.
    pub fn close(self) {
        drop(self)
    }

    /// Block until background process is done
    pub fn wait(&mut self) {
        self.handle
            .take()
            .expect("Called stop on non-running thread")
            .join()
            .expect("Could not join spawned thread");
    }

    /// Make the device turn on and off repeatedly in the background.
    /// Use `set_blink_count` to set the number of times to blink the device
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    ///
    pub fn blink(&mut self, on_time: f32, off_time: f32, on_color: Rgb, off_color: Rgb) {
        match self.blink_count {
            None => self.blinker(on_time, off_time, on_color, off_color, None),
            Some(n) => self.blinker(on_time, off_time, on_color, off_color, Some(n)),
        }
    }

    /// Set the number of times to blink the device
    /// * `n` - Number of times to blink
    pub fn set_blink_count(&mut self, n: i32) {
        self.blink_count = Some(n)
    }

    pub fn is_lit(&self) -> bool {
        self.is_active()
    }
}

///  Represents a light emitting diode (LED)
///
/// # Example
///  Connect LED as shown below, with cathode(short leg) connected to GND
///
/// ```shell
///           Resistor     LED
///  Pin 14 o--/\/\/---->|------o GND
///  ```
///

#[derive(Debug)]
pub struct LED {
    device: Arc<Mutex<OutputDevice>>,
    blinking: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    blink_count: Option<i32>,
}

impl LED {
    pub fn new(pin: u8) -> LED {
        LED {
            device: Arc::new(Mutex::new(OutputDevice::new(pin))),
            blinking: Arc::new(AtomicBool::new(false)),
            handle: None,
            blink_count: None,
        }
    }

    impl_digital_output_device!();

    /// Returns True if the device is currently active and False otherwise.
    pub fn is_lit(&self) -> bool {
        self.is_active()
    }

    /// Make the device turn on and off repeatedly in the background.
    /// Use `set_blink_count` to set the number of times to blink the device    
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    ///
    pub fn blink(&mut self, on_time: f32, off_time: f32) {
        match self.blink_count {
            None => self.blinker(on_time, off_time, None),
            Some(n) => self.blinker(on_time, off_time, Some(n)),
        }
    }
    /// Set the number of times to blink the device    
    /// * `n` - Number of times to blink
    pub fn set_blink_count(&mut self, n: i32) {
        self.blink_count = Some(n)
    }
}

/// Represents a digital buzzer component.
///
/// Connect the cathode (negative pin) of the buzzer to a ground pin;
/// connect the other side to any GPIO pin.

#[derive(Debug)]
pub struct Buzzer {
    device: Arc<Mutex<OutputDevice>>,
    blinking: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    blink_count: Option<i32>,
}

impl Buzzer {
    pub fn new(pin: u8) -> Buzzer {
        Buzzer {
            device: Arc::new(Mutex::new(OutputDevice::new(pin))),
            blinking: Arc::new(AtomicBool::new(false)),
            handle: None,
            blink_count: None,
        }
    }

    impl_digital_output_device!();

    /// Make the device turn on and off repeatedly in the background.
    /// Use `set_beep_count` to set the number of times to beep the device    
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    ///
    pub fn beep(&mut self, on_time: f32, off_time: f32) {
        match self.blink_count {
            None => self.blinker(on_time, off_time, None),
            Some(n) => self.blinker(on_time, off_time, Some(n)),
        }
    }
    /// Set the number of times to beep the device    
    /// * `n` - Number of times to beep
    pub fn set_beep_count(&mut self, n: i32) {
        self.blink_count = Some(n)
    }
}

/// Generic output device configured for software pulse-width modulation (PWM).
/// The pulse width of the signal will be 100μs with a value range of [0,100] (where 0 is a constant low and 100 is a constant high) resulting in a frequenzy of 100 Hz.
pub struct PWMOutputDevice {
    device: Arc<Mutex<OutputDevice>>,
    blinking: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    blink_count: Option<i32>,
    active_state: bool,
    inactive_state: bool,
}

macro_rules! impl_pwm_device {
    () => {
        /// Set the duty cycle of the PWM device. 0.0 is off, 1.0 is fully on.
        /// Values in between may be specified for varying levels of power in the device.
        pub fn set_value(&mut self, duty: f64) {
            self.write_state(duty)
        }
        /// Set the number of times to blink the device
        /// * `n` - Number of times to blink
        pub fn set_blink_count(&mut self, n: i32) {
            self.blink_count = Some(n)
        }

        fn blinker(
            &mut self,
            on_time: f32,
            off_time: f32,
            fade_in_time: f32,
            fade_out_time: f32,
            on_value: f32,
            off_value: f32,
            n: Option<i32>,
        ) {
            let mut sequence: Vec<(f32, f32)> = Vec::new();
            let fps = 25.0;
            // create sequence for fading in
            if fade_in_time > 0.0 {
                let frames = fps as i32 * fade_in_time as i32;
                for i in 0..frames {
                    let proportion = (i as f32 * (1.0 / fps) / fade_in_time);
                    sequence.push((
                        off_value * (1. - proportion) + on_value * proportion,
                        1.0 / fps,
                    ))
                }
            }

            // allow to stay on for on_time
            sequence.push((on_value, on_time));

            // create sequence for fading out
            if fade_out_time > 0.0 {
                let frames = fps as i32 * fade_out_time as i32;
                for i in 0..frames {
                    let proportion = (i as f32 * (1.0 / fps) / fade_out_time);
                    sequence.push((
                        on_value * (1. - proportion) + off_value * proportion,
                        1.0 / fps,
                    ))
                }
            }

            // allow to stay off for off_time
            sequence.push((off_value, off_time));

            let device = Arc::clone(&self.device);
            let blinking = Arc::clone(&self.blinking);
            let active_high = self.active_high();
            self.handle = Some(thread::spawn(move || {
                blinking.store(true, Ordering::SeqCst);

                match n {
                    Some(end) => {
                        for _ in 0..end {
                            for (value, delay) in &sequence {
                                if !blinking.load(Ordering::SeqCst) {
                                    // device.lock().unwrap().off();
                                    break;
                                }
                                let value_given_active_high =
                                    if active_high { *value } else { 1. - *value };
                                device
                                    .lock()
                                    .unwrap()
                                    .pin
                                    .set_pwm_frequency(100.0, f64::from(value_given_active_high))
                                    .unwrap();
                                thread::sleep(Duration::from_millis((delay * 1000 as f32) as u64));
                            }
                        }
                    }
                    None => loop {
                        for (value, delay) in &sequence {
                            if !blinking.load(Ordering::SeqCst) {
                                // device.lock().unwrap().off();
                                break;
                            }
                            let value_given_active_high =
                                if active_high { *value } else { 1. - *value };
                            device
                                .lock()
                                .unwrap()
                                .pin
                                .set_pwm_frequency(100.0, f64::from(value_given_active_high))
                                .unwrap();
                            thread::sleep(Duration::from_millis((delay * 1000 as f32) as u64));
                        }
                    },
                }
            }));
        }

        fn stop(&mut self) {
            self.blinking.clone().store(false, Ordering::SeqCst);
            if self.device.lock().unwrap().pin.clear_pwm().is_err() {
                println!("Could not clear pwm for pin");
            };
        }

        fn write_state(&mut self, value: f64) {
            if !(0.0..=1.0).contains(&value) {
                println!("Value must be between 0.0 and 1.0");
                return;
            }
            self.stop();
            if self.active_high() {
                self.device
                    .lock()
                    .unwrap()
                    .pin
                    .set_pwm_frequency(100.0, value)
                    .unwrap()
            } else {
                self.device
                    .lock()
                    .unwrap()
                    .pin
                    .set_pwm_frequency(100.0, 1.0 - value)
                    .unwrap()
            }
        }

        /// Set the state for active_high
        pub fn set_active_high(&mut self, value: bool) {
            if value {
                self.active_state = true;
                self.inactive_state = false;
            } else {
                self.active_state = false;
                self.inactive_state = true;
            }
        }
        /// When ``True``, the `value` property is ``True`` when the device's
        /// `pin` is high. When ``False`` the `value` property is
        /// ``True`` when the device's pin is low (i.e. the value is inverted).
        /// Be warned that changing it will invert `value` (i.e. changing this property doesn't change
        /// the device's pin state - it just changes how that state is interpreted).
        pub fn active_high(&self) -> bool {
            self.active_state
        }

        /// Turns the device on.
        pub fn on(&mut self) {
            self.write_state(1.0)
        }

        /// Turns the device off.
        pub fn off(&mut self) {
            self.write_state(0.0)
        }
    };
}

impl PWMOutputDevice {
    /// Returns a PWMOutputDevice with the pin number given
    ///
    /// * `pin` - The GPIO pin which the device is attached to
    ///  
    pub fn new(pin: u8) -> PWMOutputDevice {
        PWMOutputDevice {
            device: Arc::new(Mutex::new(OutputDevice::new(pin))),
            blinking: Arc::new(AtomicBool::new(false)),
            handle: None,
            blink_count: None,
            active_state: true,
            inactive_state: false,
        }
    }

    impl_pwm_device!();

    /// Make the device turn on and off repeatedly
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    /// * `fade_in_time` - Number of seconds to spend fading in
    /// * `fade_out_time` - Number of seconds to spend fading out
    ///
    pub fn blink(
        &mut self,
        on_time: f32,
        off_time: f32,
        fade_in_time: f32,
        fade_out_time: f32,
        on_value: f32,
        off_value: f32,
    ) {
        match self.blink_count {
            None => self.blinker(
                on_time,
                off_time,
                fade_in_time,
                fade_out_time,
                on_value,
                off_value,
                None,
            ),
            Some(n) => self.blinker(
                on_time,
                off_time,
                fade_in_time,
                fade_out_time,
                on_value,
                off_value,
                Some(n),
            ),
        }
    }

    /// Make the device fade in and out repeatedly.    
    /// * `fade_in_time` - Number of seconds to spend fading in
    /// * `fade_out_time` - Number of seconds to spend fading out
    ///
    pub fn pulse(&mut self, fade_in_time: f32, fade_out_time: f32, on_value: f32, off_value: f32) {
        self.blink(0.0, 0.0, fade_in_time, fade_out_time, on_value, off_value)
    }
}

/// Represents a light emitting diode (LED) with variable brightness.
/// A typical configuration of such a device is to connect a GPIO pin
/// to the anode (long leg) of the LED, and the cathode (short leg) to ground,
/// with an optional resistor to prevent the LED from burning out.
pub struct PWMLED(PWMOutputDevice);

impl PWMLED {
    /// Returns a PMWLED with the pin number given
    ///
    /// * `pin` - The GPIO pin which the device is attached to
    ///    
    pub fn new(pin: u8) -> PWMLED {
        PWMLED(PWMOutputDevice::new(pin))
    }

    /// Make the device turn on and off repeatedly
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    /// * `fade_in_time` - Number of seconds to spend fading in
    /// * `fade_out_time` - Number of seconds to spend fading out
    ///
    pub fn blink(
        &mut self,
        on_time: f32,
        off_time: f32,
        fade_in_time: f32,
        fade_out_time: f32,
        on_value: f32,
        off_value: f32,
    ) {
        self.0.blink(
            on_time,
            off_time,
            fade_in_time,
            fade_out_time,
            on_value,
            off_value,
        )
    }

    /// Turns the device on.
    pub fn on(&mut self) {
        self.0.on();
    }

    /// Turns the device off.
    pub fn off(&mut self) {
        self.0.off();
    }

    /// Make the device fade in and out repeatedly.
    /// * `fade_in_time` - Number of seconds to spend fading in
    /// * `fade_out_time` - Number of seconds to spend fading out
    ///
    pub fn pulse(&mut self, fade_in_time: f32, fade_out_time: f32, on_value: f32, off_value: f32) {
        self.0
            .pulse(fade_in_time, fade_out_time, on_value, off_value);
    }

    /// Set the duty cycle of the PWM device. 0.0 is off, 1.0 is fully on.
    /// Values in between may be specified for varying levels of power in the device.
    pub fn set_value(&mut self, value: f64) {
        self.0.set_value(value);
    }

    /// Set the number of times to blink the device    
    /// * `n` - Number of times to blink
    pub fn set_blink_count(&mut self, n: i32) {
        self.0.blink_count = Some(n)
    }
}

pub struct RGBPWMLED {
    red: PWMLED,
    green: PWMLED,
    blue: PWMLED,
}

impl RGBPWMLED {
    pub fn new(red: u8, green: u8, blue: u8, active_high: bool) -> RGBPWMLED {
        let mut red = PWMLED::new(red);
        let mut green = PWMLED::new(green);
        let mut blue = PWMLED::new(blue);
        red.0.set_active_high(active_high);
        green.0.set_active_high(active_high);
        blue.0.set_active_high(active_high);
        Self { red, green, blue }
    }

    /// Make the device turn on and off repeatedly
    /// * `on_time` - Number of seconds on
    /// * `off_time` - Number of seconds off
    /// * `fade_in_time` - Number of seconds to spend fading in
    /// * `fade_out_time` - Number of seconds to spend fading out
    ///
    pub fn blink(
        &mut self,
        on_time: f32,
        off_time: f32,
        fade_in_time: f32,
        fade_out_time: f32,
        on_color: Rgb,
        off_color: Rgb,
    ) {
        self.red.blink(
            on_time,
            off_time,
            fade_in_time,
            fade_out_time,
            on_color.red,
            off_color.red,
        );
        self.green.blink(
            on_time,
            off_time,
            fade_in_time,
            fade_out_time,
            on_color.green,
            off_color.green,
        );
        self.blue.blink(
            on_time,
            off_time,
            fade_in_time,
            fade_out_time,
            on_color.blue,
            off_color.blue,
        );
    }

    /// Turns all LEDs on.
    pub fn on(&mut self) {
        self.red.on();
        self.green.on();
        self.blue.on();
    }

    /// Turns all LEDs off.
    pub fn off(&mut self) {
        self.red.off();
        self.green.off();
        self.blue.off();
    }

    /// Stops device.
    pub fn stop(&mut self) {
        self.red.0.stop();
        self.green.0.stop();
        self.blue.0.stop();
    }

    /// Make the device fade in and out repeatedly.
    /// * `fade_in_time` - Number of seconds to spend fading in
    /// * `fade_out_time` - Number of seconds to spend fading out
    ///
    pub fn pulse(&mut self, fade_in_time: f32, fade_out_time: f32, on_color: Rgb, off_color: Rgb) {
        self.red
            .pulse(fade_in_time, fade_out_time, on_color.red, off_color.red);
        self.green
            .pulse(fade_in_time, fade_out_time, on_color.green, off_color.green);
        self.blue
            .pulse(fade_in_time, fade_out_time, on_color.blue, off_color.blue);
    }

    /// Set the duty cycle of the PWM device. 0.0 is off, 1.0 is fully on.
    /// Values in between may be specified for varying levels of power in the device.
    pub fn set_value(&mut self, rgb: Rgb) {
        if !(0.0..=1.0).contains(&rgb.red)
            || !(0.0..=1.0).contains(&rgb.green)
            || !(0.0..=1.0).contains(&rgb.blue)
        {
            panic!("Invalid colour: {:?}", rgb);
        }
        self.red.set_value(rgb.red.into());
        self.green.set_value(rgb.green.into());
        self.blue.set_value(rgb.blue.into());
    }

    /// Set the number of times to blink the device    
    /// * `n` - Number of times to blink
    pub fn set_blink_count(&mut self, n: i32) {
        self.red.0.blink_count = Some(n);
        self.green.0.blink_count = Some(n);
        self.blue.0.blink_count = Some(n);
    }

    /// Returns whether any of red, green or blue are active.
    pub fn is_active(&self) -> bool {
        self.red.0.device.lock().unwrap().is_active()
            || self.green.0.device.lock().unwrap().is_active()
            || self.blue.0.device.lock().unwrap().is_active()
    }

    /// Switches off if active and on if inactive.
    pub fn toggle(&mut self) {
        if self.is_active() {
            self.off()
        } else {
            self.on()
        }
    }
}

struct MotorCompositeDevice(PWMOutputDevice, PWMOutputDevice);

///  Represents a generic motor connected
///  to a bi-directional motor driver circuit (i.e. an H-bridge).
///  Attach an H-bridge motor controller to your Pi; connect a power source (e.g. a battery pack or the 5V pin)
///  to the controller; connect the outputs of the controller board to the two terminals of the motor; connect the inputs of the controller board to two GPIO pins.
pub struct Motor {
    devices: MotorCompositeDevice,
    speed: f64,
}

impl Motor {
    /// creates a new Motor instance
    /// * `forward_pin` - The GPIO pin that the forward input of the motor driver chip is connected to
    /// * `backward` - The GPIO pin that the backward input of the motor driver chip is connected to
    pub fn new(forward_pin: u8, backward_pin: u8) -> Motor {
        let forward = PWMOutputDevice::new(forward_pin);
        let backward = PWMOutputDevice::new(backward_pin);
        Motor {
            devices: MotorCompositeDevice(forward, backward),
            speed: 1.0,
        }
    }

    /// Drive the motor forwards at the current speed.
    /// You can change the speed using `set_speed` before calling `forward`
    pub fn forward(&mut self) {
        self.devices.1.off();
        self.devices.0.set_value(self.speed);
    }

    /// Drive the motor backwards.
    /// You can change the speed using `set_speed` before calling `backward`
    pub fn backward(&mut self) {
        self.devices.0.off();
        self.devices.1.set_value(self.speed);
    }

    /// Stop the motor.
    pub fn stop(&mut self) {
        self.devices.0.off();
        self.devices.1.off();
    }

    /// The speed at which the motor should turn.
    /// Can be any value between 0.0 (stopped) and the default 1.0 (maximum speed)
    pub fn set_speed(&mut self, speed: f64) {
        if !(0.0..=1.0).contains(&speed) {
            println!("Speed must be between 0.0 and 1.0");
            return;
        }
        self.speed = speed
    }
}

/// Represents a PWM-controlled servo motor connected to a GPIO pin.
//reference :https://github.com/golemparts/rppal/blob/master/examples/gpio_servo_softpwm.rs
pub struct Servo {
    pin: IoPin,
    min_pulse_width: u64,
    max_pulse_width: u64,
    frame_width: u64,
}

impl Servo {
    /// Returns a Servo with the pin number given with default `min_pulse_width` of 1ms,
    /// `max_pulse_width` of 2ms and `frame_width` of 20ms
    ///
    /// * `pin` - The GPIO pin which the device is attached to
    ///  
    pub fn new(pin: u8) -> Servo {
        match Gpio::new() {
            Err(e) => panic!("{:?}", e),
            Ok(gpio) => match gpio.get(pin) {
                Err(e) => panic!("{:?}", e),
                Ok(pin) => Servo {
                    pin: pin.into_io(Mode::Output),
                    min_pulse_width: 1000,
                    max_pulse_width: 2000,
                    frame_width: 20,
                },
            },
        }
    }

    /// Set the servo to its minimum position.
    pub fn min(&mut self) {
        if self
            .pin
            .set_pwm(
                Duration::from_millis(self.frame_width),
                Duration::from_micros(self.min_pulse_width),
            )
            .is_err()
        {
            println!("Failed to set servo to minimum position")
        }
    }

    /// Set the servo to its maximum position.
    pub fn max(&mut self) {
        if self
            .pin
            .set_pwm(
                Duration::from_millis(self.frame_width),
                Duration::from_micros(self.max_pulse_width),
            )
            .is_err()
        {
            println!("Failed to set servo to maximum position")
        }
    }

    /// Set the servo to its neutral position.
    pub fn mid(&mut self) {
        let mid_value = (self.min_pulse_width + self.max_pulse_width) / 2;
        if self
            .pin
            .set_pwm(
                Duration::from_millis(self.frame_width),
                Duration::from_micros(mid_value),
            )
            .is_err()
        {
            println!("Failed to set servo to neutral position")
        }
    }

    /// Set servo to any position between min and max.
    /// value must be between -1 (the minimun position) and +1 (the maximum position).
    pub fn set_position(&mut self, value: f64) {
        if value >= -1.0 && value <= 1.0 {
            // Map value form [-1, 1] to [min_pulse_width, max_pulse_width] linearly
            let range: f64 = (self.max_pulse_width - self.min_pulse_width) as f64;
            let pulse_width: u64 =
                self.min_pulse_width + (((value + 1.0) / 2.0) * range).round() as u64;
            if self
                .pin
                .set_pwm(
                    Duration::from_millis(self.frame_width),
                    Duration::from_micros(pulse_width),
                )
                .is_err()
            {
                println!("Failed to set servo to a new position");
            }
        } else {
            println!("set_position value must be between -1 and 1");
        }
    }

    /// Set the servo's minimum pulse width
    pub fn set_min_pulse_width(&mut self, value: u64) {
        if value >= self.max_pulse_width {
            println!("min_pulse_width must be less than max_pulse_width");
        } else {
            self.min_pulse_width = value;
        }
    }

    /// Set the servo's maximum pulse width
    pub fn set_max_pulse_width(&mut self, value: u64) {
        if value >= self.frame_width * 1000 {
            println!("max_pulse_width must be less than frame_width");
        } else {
            self.max_pulse_width = value;
        }
    }

    /// Set the servo's frame width(The time between control pulses, measured in milliseconds.)
    pub fn set_frame_width(&mut self, value: u64) {
        self.frame_width = value;
    }

    /// Get the servo's minimum pulse width
    pub fn get_min_pulse_width(&mut self) -> u64 {
        self.min_pulse_width
    }

    /// Get the servo's maximum pulse width
    pub fn get_max_pulse_width(&mut self) -> u64 {
        self.max_pulse_width
    }

    /// Get the servo's frame width(The time between control pulses, measured in milliseconds.)
    pub fn get_frame_width(&mut self) -> u64 {
        self.frame_width
    }

    pub fn detach(&mut self) {
        if self.pin.clear_pwm().is_err() {
            println!("Failed to detach servo")
        }
    }
}
