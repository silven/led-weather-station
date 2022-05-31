use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
};
use rpi_led_matrix::{LedCanvas, LedMatrix, LedMatrixOptions, LedRuntimeOptions};
use signal_hook::{consts::TERM_SIGNALS, flag};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub trait Screen {
    fn left(&mut self);
    fn right(&mut self);
    fn click(&mut self);
    fn draw(&mut self, canvas: &mut LedCanvas);
}

mod background;
mod mailbox;
mod rotary;
mod waves;
use rotary::InputEvent;

fn main() {
    let term = setup_signal_trapping();
    let irx = start_input_thread(&term);

    let mut background = background::BackgroundScreen::new(Arc::clone(&term));

    let matrix = setup_matrix();

    let mut canvas = matrix.offscreen_canvas();
    canvas.clear();
    canvas = matrix.swap(canvas);

    let mut wave = waves::WaveScreen::new(&canvas);

    let border_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb888::WHITE)
        .stroke_width(1)
        .build();

    let selection_mode_border =
        Rectangle::new(Point::new(0, 0), Size::new(64, 32)).into_styled(border_style);

    let mut screen_idx = 0usize;

    let mut selection_mode = false;
    let screens = [
        &mut background as &mut dyn Screen,
        &mut wave as &mut dyn Screen,
    ];

    while !term.load(Ordering::Relaxed) {
        canvas.clear();

        if let Ok(evt) = irx.try_recv() {
            if selection_mode {
                let ds = match evt {
                    InputEvent::Left => -1,
                    InputEvent::Right => 1,
                    _ => {
                        selection_mode = false;
                        0
                    }
                };

                let ilen = screens.len() as isize;
                screen_idx = ((screen_idx as isize + ds) % ilen + ilen) as usize % screens.len();
            } else {
                match evt {
                    InputEvent::Left => screens[screen_idx].left(),
                    InputEvent::Right => screens[screen_idx].right(),
                    InputEvent::Click => screens[screen_idx].click(),
                    InputEvent::LongPress => selection_mode = true,
                };
            }
        }

        screens[screen_idx].draw(&mut canvas);

        if selection_mode {
            selection_mode_border
                .draw(&mut canvas)
                .expect("draw border");
        }

        canvas = matrix.swap(canvas);
        thread::sleep(Duration::from_millis(1));
    }

    // Cleanup
    canvas.clear();
    canvas = matrix.swap(canvas);
}

fn setup_matrix() -> LedMatrix {
    let mut options = LedMatrixOptions::new();
    options.set_hardware_mapping("adafruit-hat-pwm");
    options.set_brightness(100);
    options.set_rows(32);
    options.set_cols(64);

    let mut rt_options = LedRuntimeOptions::new();
    rt_options.set_gpio_slowdown(0);

    LedMatrix::new(Some(options), Some(rt_options)).expect("init matrix")
}

fn setup_signal_trapping() -> Arc<AtomicBool> {
    let term = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term)).expect("register1");
        flag::register(*sig, Arc::clone(&term)).expect("register2");
    }
    term
}

fn start_input_thread(term: &Arc<AtomicBool>) -> Receiver<InputEvent> {
    let (itx, irx) = channel::<InputEvent>();
    let term_ = Arc::clone(term);

    thread::spawn(move || {
        let mut listener = rotary::RotaryEncoder::new(itx);
        listener.poll_loop(term_);
    });

    irx
}
