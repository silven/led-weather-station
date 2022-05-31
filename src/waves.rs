use rand::prelude::*;
use rpi_led_matrix::{LedCanvas, LedColor};

fn xy_to_index(width: i32, x: i32, y: i32) -> usize {
    assert!(width > 0);
    assert!(x >= 0);
    assert!(y >= 0);
    (y * width + x) as usize
}

pub struct WaveScreen {
    current_map: Vec<f32>,
    last_map: Vec<f32>,
    hue: f32,
}

fn clamp(f: f32) -> u8 {
    ((f * 255.0) as u8).min(255).max(0)
}

fn shift_color(c: (f32, f32, f32), hue: f32) -> LedColor {
    use rulinalg::matrix;
    let orig_color = matrix![c.0, c.1, c.2];

    let sin = hue.sin();
    let cos = hue.cos();

    let _1d3: f32 = 1.0 / 3.0;
    let sqrt1d3 = _1d3.sqrt();

    let rotation_matrix = matrix![
        cos + (1.0 - cos) / 3.0,             _1d3 * (1.0 - cos) - sqrt1d3 * sin,  _1d3 * (1.0 - cos) + sqrt1d3 * sin;
        _1d3 * (1.0 - cos) + sqrt1d3 * sin,  cos + _1d3 * (1.0 - cos),            _1d3 * (1.0 - cos) - sqrt1d3 * sin;
        _1d3 * (1.0 - cos) - sqrt1d3 * sin,  _1d3 * (1.0 - cos) + sqrt1d3 * sin,  _1d3 * (1.0 - cos) + sqrt1d3 * sin
    ];

    let result = (orig_color * rotation_matrix).into_vec();

    LedColor {
        red: clamp(result[0]),
        green: clamp(result[1]),
        blue: clamp(result[2]),
    }
}

impl WaveScreen {
    pub fn new(canvas: &LedCanvas) -> Self {
        let (width, height) = canvas.canvas_size();
        let map = (0..(width * height)).map(|_| rand::random()).collect();

        Self {
            current_map: map,
            last_map: vec![0.0; (width * height) as usize],
            hue: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.current_map = (0..self.current_map.len())
            .map(|_| rand::random())
            .collect();
    }

    fn draw_pixels(&self, canvas: &mut LedCanvas) {
        let (width, height) = canvas.canvas_size();
        let map = &self.current_map;
        for y in 0..height {
            for x in 0..width {
                let i = xy_to_index(width, x, y);

                let color = (
                    map[i].powf(4.0 + (map[i] * 0.5)) * map[i].cos(),
                    map[i].powf(3.0 + (map[i] * 0.5)) * map[i].sin(),
                    map[i].powf(2.0 + (map[i] * 0.5)),
                );

                let shifted = shift_color(color, self.hue);

                canvas.set(x, y, &shifted);
            }
        }
    }

    pub fn draw(&mut self, canvas: &mut LedCanvas) {
        std::mem::swap(&mut self.current_map, &mut self.last_map);

        let (width, height) = canvas.canvas_size();
        for y in 0..height {
            for x in 0..width {
                let i = xy_to_index(width, x, y);
                let last_value = self.last_map[i];

                self.current_map[i] = last_value * (0.96 + 0.02 * rand::random::<f32>());

                if last_value <= (0.18 + 0.04 * rand::random::<f32>()) {
                    let mut n = 0;

                    for u in -1..=1 {
                        for v in -1..=1 {
                            if u == 0 && u == 0 {
                                continue;
                            }

                            let n_x = ((x + u) % width).abs();
                            let n_y = ((y + v) % height).abs();

                            let n_i = xy_to_index(width, n_x, n_y);
                            let n_last_value = self.last_map[n_i];

                            if n_last_value >= (0.5 + 0.04 * rand::random::<f32>()) {
                                n += 1;
                                self.current_map[i] +=
                                    n_last_value * (0.8 + 0.4 * rand::random::<f32>());
                            }
                        }
                    }

                    if n > 0 {
                        self.current_map[i] *= 1.0 / (n as f32);
                    }

                    if self.current_map[i] > 1.0 {
                        self.current_map[i] = 1.0;
                    }
                }
            }
        }

        self.draw_pixels(canvas);
    }
}

impl crate::Screen for WaveScreen {
    fn left(&mut self) {
        self.hue -= 0.1;
    }

    fn right(&mut self) {
        self.hue += 0.1;
    }

    fn click(&mut self) {
        self.reset();
    }

    fn draw(&mut self, canvas: &mut LedCanvas) {
        self.draw(canvas);
    }
}
