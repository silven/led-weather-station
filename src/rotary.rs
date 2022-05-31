use rppal::gpio::{Gpio, Level, Trigger};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

// BCM MODE
const LEFT_PIN: u8 = 8;
const RIGHT_PIN: u8 = 7;
const BUTTON_PIN: u8 = 19;

pub struct RotaryEncoder {
    gpio: Gpio,
    tx: Sender<InputEvent>,
    state: (Level, Level),
}

#[derive(PartialEq, Debug)]
pub enum InputEvent {
    Left,
    Right,
    Click,
    LongPress,
}

#[derive(PartialEq, Eq, Debug)]
enum Direction {
    Unknown,
    Left,
    Right,
}

impl RotaryEncoder {
    pub fn new(tx: Sender<InputEvent>) -> Self {
        let gpio = Gpio::new().expect("acees gpio");
        RotaryEncoder {
            gpio: gpio,
            tx: tx,
            state: (Level::Low, Level::Low),
        }
    }

    fn handle_rotation(&mut self, direction: &mut Direction, left: Level, right: Level) {
        use Direction::{Left, Right};
        use Level::{High, Low};

        let new_state = (left, right);

        match self.state {
            // 00 can go to either 01 or 10, this gives us a direction
            (Low, Low) => {
                if new_state == (Low, High) {
                    *direction = Right;
                } else if new_state == (High, Low) {
                    *direction = Left;
                }
            },

            // 01 can go to either 11 or 00
            (Low, High) => {
                if new_state == (High, High) {
                    *direction = Right;
                } else if new_state == (Low, Low) {
                    if *direction == Left {
                        self.tx.send(InputEvent::Left);
                    }
                }
            },

            // 10 can go to either 11 or 00
            (High, Low) => {
                if new_state == (High, High) {
                    *direction = Left;
                } else if new_state == (Low, Low) {
                    if *direction == Right {
                        self.tx.send(InputEvent::Right);
                    }
                }
            },

            // 11 can go to either 01 or 10
            (High, High) => {
                if new_state == (Low, High) {
                    *direction = Left;
                } else if new_state == (High, Low) {
                    *direction = Right;
                // If we're somehow at 00, we missed an "edge".
                // But we can use the Direction value to deduce where we came from
                } else if new_state == (Low, Low) {
                    if *direction == Left {
                        self.tx.send(InputEvent::Left);
                    } else if *direction == Right {
                        self.tx.send(InputEvent::Right);
                    }
                }
            }
        }

        self.state = new_state;
    }

    pub fn poll_loop(&mut self, terminate: Arc<AtomicBool>) -> Result<(), rppal::gpio::Error> {
        let timeout = Some(Duration::from_secs(1));

        let mut left = self.gpio.get(LEFT_PIN)?.into_input_pullup();
        left.set_interrupt(Trigger::Both);

        let mut right = self.gpio.get(RIGHT_PIN)?.into_input_pullup();
        right.set_interrupt(Trigger::Both);

        let mut button = self.gpio.get(BUTTON_PIN)?.into_input_pullup();
        button.set_interrupt(Trigger::FallingEdge);

        // State
        let mut rotation = Direction::Unknown;

        // input grouping
        let reset = false;
        let pins = [&button, &left, &right];
        while !terminate.load(Ordering::Relaxed) {
            if let Ok(Some((pin, level))) = self.gpio.poll_interrupts(&pins, reset, timeout) {
                match pin.pin() {
                    LEFT_PIN | RIGHT_PIN => {
                        self.handle_rotation(&mut rotation, left.read(), right.read());
                    }
                    BUTTON_PIN if level == Level::Low => {
                        let button_down = Instant::now();
                        let mut sent = false;

                        while button.read() == Level::Low {
                            std::thread::sleep(Duration::from_millis(100));
                            if button_down.elapsed().as_millis() > 1000 {
                                self.tx.send(InputEvent::LongPress);
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            self.tx.send(InputEvent::Click);
                        }
                    }
                    _ => {} // do nothing
                };
            }
        }

        Ok(())
    }
}
