use rpi_led_matrix::LedCanvas;

pub trait Screen {
    fn left(&mut self);
    fn right(&mut self);
    fn click(&mut self);
    fn draw(&mut self, canvas: &mut LedCanvas);
}

mod background;
mod waves;
mod maze;

pub use background::BackgroundScreen;
pub use waves::WaveScreen;
pub use maze::MazeScreen;