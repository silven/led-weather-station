use std::collections::{VecDeque, HashMap};

use maze_generator::ellers_algorithm::EllersGenerator;
use maze_generator::growing_tree::GrowingTreeGenerator;
use maze_generator::prims_algorithm::PrimsGenerator;
use rand;
use rand::{Rng, prelude::ThreadRng};
use rpi_led_matrix::{LedCanvas, LedColor};

use super::Screen;

use maze_generator::prelude::*;
use maze_generator::recursive_backtracking::RbGenerator;

pub struct MazeScreen {
    maze: Maze,
    queue: VecDeque<(Coordinates, Option<Direction>)>,
    visited: HashMap<Coordinates, Option<Direction>>,
    done: bool,
}

impl MazeScreen {
    pub fn new(canvas: &LedCanvas) -> Self {
        let (width, height) = canvas.canvas_size();

        let mut generator = RbGenerator::new(Some([13; 32]));
        let maze = generator.generate(width/2, height/2).unwrap();

        let mut queue = VecDeque::new();
        queue.push_back((maze.start, None));

        Self { maze, queue, visited: HashMap::new(), done: false }
    }

    fn reset(&mut self, rng: &mut impl Rng) {
        use rand::prelude::SliceRandom;

        self.visited.clear();
        self.done = false;

        let (width, height) = self.maze.size;
        let seed = Some(rng.gen());

        let mut generators = [
            &mut EllersGenerator::new(seed) as &mut dyn Generator,
            &mut RbGenerator::new(seed) as &mut dyn Generator,
            &mut PrimsGenerator::new(seed) as &mut dyn Generator,
            &mut GrowingTreeGenerator::new(seed) as &mut dyn Generator,
        ];

        self.maze = generators.choose_mut(rng).unwrap().generate(width, height).unwrap();

        self.queue.push_back((self.maze.start, None));
    }

    fn draw_maze(&mut self, canvas: &mut LedCanvas) {

        let outline = LedColor {red: 120, green: 120, blue: 120 };
        let (w, h) = self.maze.size;
        for y in 0..h {
            for x in 0..w {
                if let Some(f) = self.maze.get_field(&(x, y).into()) {
                    canvas.set(x*2, y*2, &outline);

                    for d in Direction::all() {
                        if f.has_passage(&d) && self.maze.get_field(&f.coordinates.next(&d)).is_some() {
                            let (dx, dy) = match d {
                                Direction::North => (0, -1),
                                Direction::East => (1, 0),
                                Direction::South => (0, 1),
                                Direction::West => (-1, 0),
                            };
                            canvas.set((x*2)+dx, (y*2)+dy, &outline);
                        }
                    }
                }
            }
        }

        let visited = LedColor {red: 255, green: 255, blue: 255 };
        //let start  = LedColor {red: 0, green: 255, blue: 0 };
        let goal  = LedColor {red: 255, green: 0, blue: 0 };

        for (coord, entrance) in &self.visited {
            canvas.set(coord.x*2, coord.y*2, &visited);

            if let Some(way_in) = entrance {
                let (dx, dy) = match way_in {
                    Direction::North => (0, -1),
                    Direction::East => (1, 0),
                    Direction::South => (0, 1),
                    Direction::West => (-1, 0),
                };
                canvas.set((coord.x*2)+dx, (coord.y*2)+dy, &visited);
            }
        }

        //canvas.set(self.maze.start.x*2, self.maze.start.y*2, &start);
        canvas.set(self.maze.goal.x*2, self.maze.goal.y*2, &goal);

    }
}


impl Screen for MazeScreen {
    fn left(&mut self) {
    }

    fn right(&mut self) {
    }

    fn click(&mut self) {
        let mut rng = rand::thread_rng();
        self.reset(&mut rng);
    }

    fn draw(&mut self, canvas: &mut LedCanvas) {
        let mut rng = rand::thread_rng();

        if self.done {
            std::thread::sleep(std::time::Duration::from_millis(400));
            self.reset(&mut rng);
        } else if self.visited.contains_key(&self.maze.goal) {
            self.done = true;
        }

        self.draw_maze(canvas);

        if let Some((to_explore, way_in)) = self.queue.pop_front() {

            if let Some(field) = self.maze.get_field(&to_explore) {
                self.visited.insert(field.coordinates, way_in);

                for d in Direction::gen_random_order(&mut rng) {
                    if field.has_passage(&d) {
                        let next = field.coordinates.next(&d);
                        
                        if !self.visited.contains_key(&next) {
                            self.queue.push_front((next, Some(d.opposite())));
                        }
                    }
                }
            }
        }
        
    }
}