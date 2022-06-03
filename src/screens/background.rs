use chrono::Local;
use embedded_graphics::prelude::*;
use embedded_graphics::{
    image::Image,
    mono_font::{iso_8859_1::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    text::{Alignment, Text},
};
use image::io::Reader as ImageReader;
use image::{imageops::FilterType, ImageOutputFormat};
use linux_embedded_hal::Delay;
use linux_embedded_hal::I2cdev;
use sensor_scd30::Measurement;
use sensor_scd30::Scd30;
use std::collections::LinkedList;
use std::io::Cursor;
use std::thread;
use std::time::Duration;
use std::{
    error::Error,
    sync::{atomic::Ordering, Arc},
};
use tinybmp::DynamicBmp;

use std::sync::atomic::AtomicBool;

static DEFAULT_BACKGROUND: &[u8] = include_bytes!("../../sakura-bg.bmp");

use crate::mailbox::Mailbox;

pub struct BackgroundScreen {
    buffers: LinkedList<Vec<u8>>,
    rx: Mailbox<Measurement>,
    default: DynamicBmp<'static, Rgb888>,
    font_style: MonoTextStyle<'static, Rgb888>,
    sensor_string: String,
    clock_string: String,
    render_state: (i32, i32),
}

fn sensor_thread(i2c: I2cdev, tx: Mailbox<Measurement>, cancel: Arc<AtomicBool>) {
    let mut scd = Scd30::new(i2c, Delay {}).unwrap();

    let pressure_compensation = 1004; // hPa
    scd.set_measurement_interval(2).unwrap();
    scd.start_continuous(pressure_compensation).unwrap();

    while !cancel.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(1));
        if let Err(e) = scd.data_ready() {
            eprintln!("E: {:?}", e);
            continue;
        }

        match scd.read_data() {
            Ok(m) => {
                tx.put(m).expect("write measurement");
            }
            Err(e) => eprintln!("Polled {:?}", e),
        }
    }

    scd.stop_continuous().expect("stop sensor");
}

fn fetch_background(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let body = minreq::get(url).send()?.into_bytes();

    let orig = ImageReader::new(Cursor::new(body))
        .with_guessed_format()?
        .decode()?;
    let resized = orig.resize_to_fill(64, 32, FilterType::Triangle);

    let mut buffer = Vec::with_capacity((resized.width() * resized.height() * 3) as usize);
    resized.write_to(&mut Cursor::new(&mut buffer), ImageOutputFormat::Bmp)?;

    Ok(buffer)
}

impl BackgroundScreen {
    pub fn new(term: Arc<AtomicBool>) -> Self {
        let rx = Mailbox::new();
        let i2c = I2cdev::new("/dev/i2c-1").expect("No i2c device");

        let tx = rx.clone();
        thread::spawn(move || {
            sensor_thread(i2c, tx, term);
        });

        let src = [
            "https://c4.wallpaperflare.com/wallpaper/765/580/971/digital-art-pixel-art-pixels-landscape-wallpaper-preview.jpg",
            "https://c4.wallpaperflare.com/wallpaper/406/189/125/digital-art-pixel-art-pixelated-pixels-wallpaper-preview.jpg",
            "https://wallpaperaccess.com/full/2122578.jpg",
            //"https://art.pixilart.com/a72d2ef88ded5fd.png",    
            //"https://hyperpad-forum.s3.amazonaws.com/assets/559e7b8c-f34a-4ab1-a326-d3e241ce76cd.png",
        ];

        let mut images = LinkedList::new();
        for img in src {
            match fetch_background(img) {
                Ok(buffer) => images.push_back(buffer),
                Err(e) => eprintln!("Background download err: `{}`", e),
            }
        }

        let default = DynamicBmp::from_slice(&DEFAULT_BACKGROUND).expect("Parse bmp data");
        let font_style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);

        BackgroundScreen {
            buffers: images,
            rx: rx,
            default: default,
            font_style: font_style,
            sensor_string: "Loading...".to_string(),
            clock_string: "HH:MM:SS".to_string(),
            render_state: (0, 0),
        }
    }

    fn next(&mut self) {
        if let Some(value) = self.buffers.pop_front() {
            self.buffers.push_back(value);
        }
    }

    fn prev(&mut self) {
        if let Some(value) = self.buffers.pop_back() {
            self.buffers.push_front(value);
        }
    }

    fn draw(&mut self, canvas: &mut rpi_led_matrix::LedCanvas) {
        use std::fmt::Write; // allow write! into &mut String

        if let Some(img) = self.buffers.front() {
            let bmp = DynamicBmp::from_slice(img).unwrap();

            Image::new(&bmp, Point::new(0, 0))
                .draw(canvas)
                .expect("cannot draw background");
        } else {
            Image::new(&self.default, Point::new(0, 0))
                .draw(canvas)
                .expect("cannot draw background");
        }

        self.rx.if_new(|m| {
            if !m.co2.is_nan() {
                self.sensor_string.clear();

                write!(
                    &mut self.sensor_string,
                    "Co2: {} ppm, T: {:.1} ºC, Hum: {} %RH",
                    m.co2 as isize, m.temp, m.rh as isize
                )
                .expect("format sensor string");
            }
        });

        self.clock_string.clear();
        write!(
            &mut self.clock_string,
            "{}",
            Local::now().format("%H:%M:%S")
        );

        let (mut x, mut dx) = self.render_state;

        dx += 1;
        if dx == 7 {
            x += 1;
            dx = 0;

            if x as usize % (self.sensor_string.chars().count() * 8) == 0 {
                x = 0;
            }
        }

        Text::new(&self.sensor_string, Point::new(64 - x, 10), self.font_style)
            .draw(canvas)
            .expect("Could not draw");

        Text::with_alignment(
            &self.clock_string,
            Point::new(32, 20),
            self.font_style,
            Alignment::Center,
        )
        .draw(canvas)
        .expect("Could not draw");

        self.render_state = (x, dx);
    }
}

impl crate::Screen for BackgroundScreen {
    fn left(&mut self) {
        self.prev();
    }

    fn right(&mut self) {
        self.next();
    }

    fn click(&mut self) {
        // Do nothing
    }

    fn draw(&mut self, canvas: &mut rpi_led_matrix::LedCanvas) {
        self.draw(canvas);
    }
}

fn wrap(value: usize, delta: isize, size: usize) -> usize {
    let ilen = size as isize;

    ((value as isize + delta) % ilen + ilen) as usize % size
}
