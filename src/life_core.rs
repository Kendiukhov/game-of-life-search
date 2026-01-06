use ahash::AHasher;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Seed {
    pub width: usize,
    pub height: usize,
    pub live_cells: Vec<(usize, usize)>,
}

impl Seed {
    pub fn random<R: Rng>(width: usize, height: usize, density: f32, rng: &mut R) -> Self {
        let mut live_cells = Vec::new();
        for y in 0..height {
            for x in 0..width {
                if rng.gen::<f32>() < density {
                    live_cells.push((x, y));
                }
            }
        }
        Seed {
            width,
            height,
            live_cells,
        }
    }

    pub fn to_state(&self) -> LifeState {
        let mut state = LifeState::new(self.width, self.height);
        for &(x, y) in &self.live_cells {
            if x < self.width && y < self.height {
                state.set_alive(x, y, true);
            }
        }
        state.recompute_bbox();
        state
    }

    pub fn density(&self) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }
        self.live_cells.len() as f32 / (self.width * self.height) as f32
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BBox {
    pub min_x: usize,
    pub max_x: usize,
    pub min_y: usize,
    pub max_y: usize,
}

impl BBox {
    pub fn width(&self) -> usize {
        self.max_x.saturating_sub(self.min_x) + 1
    }

    pub fn height(&self) -> usize {
        self.max_y.saturating_sub(self.min_y) + 1
    }

    pub fn area(&self) -> usize {
        self.width() * self.height()
    }

    pub fn diag(&self) -> f64 {
        let w = self.width() as f64;
        let h = self.height() as f64;
        (w * w + h * h).sqrt()
    }

    pub fn touches_bounds(&self, width: usize, height: usize) -> bool {
        self.min_x == 0 || self.min_y == 0 || self.max_x + 1 >= width || self.max_y + 1 >= height
    }
}

#[derive(Clone, Debug)]
pub struct LifeState {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<u8>,
    bbox: Option<BBox>,
}

impl LifeState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![0; width * height],
            bbox: None,
        }
    }

    pub fn from_seed(seed: &Seed) -> Self {
        seed.to_state()
    }

    pub fn population(&self) -> usize {
        self.cells.iter().map(|&v| v as usize).sum()
    }

    pub fn bbox(&self) -> Option<BBox> {
        self.bbox.clone()
    }

    pub fn is_alive(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.cells[self.idx(x, y)] == 1
    }

    pub fn set_alive(&mut self, x: usize, y: usize, alive: bool) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.idx(x, y);
        self.cells[idx] = alive as u8;
    }

    pub fn step(&mut self) -> StepMetrics {
        if self.width == 0 || self.height == 0 {
            return StepMetrics {
                pop: 0,
                flips: 0,
                bbox: None,
                hash: 0,
            };
        }

        let (x_start, x_end, y_start, y_end) = match &self.bbox {
            Some(bb) => (
                bb.min_x.saturating_sub(1),
                min(bb.max_x + 1, self.width - 1),
                bb.min_y.saturating_sub(1),
                min(bb.max_y + 1, self.height - 1),
            ),
            None => (0, self.width - 1, 0, self.height - 1),
        };

        let mut next = vec![0u8; self.cells.len()];
        let mut new_bbox: Option<BBox> = None;
        let mut flips = 0usize;
        let mut pop = 0usize;

        for y in y_start..=y_end {
            for x in x_start..=x_end {
                let idx = self.idx(x, y);
                let alive = self.cells[idx] == 1;
                let neighbors = self.count_neighbors(x, y, x_start, x_end, y_start, y_end);
                let new_alive = match (alive, neighbors) {
                    (true, 2 | 3) => true,
                    (false, 3) => true,
                    _ => false,
                };

                if new_alive {
                    next[idx] = 1;
                    pop += 1;
                    new_bbox = Some(match new_bbox {
                        Some(bb) => BBox {
                            min_x: min(bb.min_x, x),
                            max_x: max(bb.max_x, x),
                            min_y: min(bb.min_y, y),
                            max_y: max(bb.max_y, y),
                        },
                        None => BBox {
                            min_x: x,
                            max_x: x,
                            min_y: y,
                            max_y: y,
                        },
                    });
                }
                if alive != new_alive {
                    flips += 1;
                }
            }
        }

        self.cells = next;
        self.bbox = new_bbox.clone();
        let hash = self.canonical_hash();

        StepMetrics {
            pop,
            flips,
            bbox: new_bbox,
            hash,
        }
    }

    pub fn canonical_hash(&self) -> u64 {
        let mut hasher = AHasher::default();
        if let Some(bb) = &self.bbox {
            hasher.write(&bb.width().to_le_bytes());
            hasher.write(&bb.height().to_le_bytes());
            let origin_x = bb.min_x;
            let origin_y = bb.min_y;
            for y in bb.min_y..=bb.max_y {
                for x in bb.min_x..=bb.max_x {
                    if self.is_alive(x, y) {
                        let nx = x - origin_x;
                        let ny = y - origin_y;
                        (nx, ny).hash(&mut hasher);
                    }
                }
            }
        }
        hasher.finish()
    }

    pub fn to_ascii(&self, padding: usize) -> String {
        let bb = self.bbox.clone().unwrap_or(BBox {
            min_x: 0,
            max_x: self.width.saturating_sub(1),
            min_y: 0,
            max_y: self.height.saturating_sub(1),
        });
        let min_x = bb.min_x.saturating_sub(padding);
        let max_x = min(bb.max_x + padding, self.width.saturating_sub(1));
        let min_y = bb.min_y.saturating_sub(padding);
        let max_y = min(bb.max_y + padding, self.height.saturating_sub(1));

        let mut lines = Vec::new();
        for y in min_y..=max_y {
            let mut line = String::new();
            for x in min_x..=max_x {
                line.push(if self.is_alive(x, y) { 'O' } else { '.' });
            }
            lines.push(line);
        }
        lines.join("\n")
    }

    fn recompute_bbox(&mut self) {
        let mut bbox: Option<BBox> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_alive(x, y) {
                    bbox = Some(match bbox {
                        Some(bb) => BBox {
                            min_x: min(bb.min_x, x),
                            max_x: max(bb.max_x, x),
                            min_y: min(bb.min_y, y),
                            max_y: max(bb.max_y, y),
                        },
                        None => BBox {
                            min_x: x,
                            max_x: x,
                            min_y: y,
                            max_y: y,
                        },
                    });
                }
            }
        }
        self.bbox = bbox;
    }

    fn count_neighbors(
        &self,
        x: usize,
        y: usize,
        x_start: usize,
        x_end: usize,
        y_start: usize,
        y_end: usize,
    ) -> u8 {
        let mut count = 0u8;
        let xi = x as isize;
        let yi = y as isize;
        let xs = x_start as isize;
        let xe = x_end as isize;
        let ys = y_start as isize;
        let ye = y_end as isize;

        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = xi + dx;
                let ny = yi + dy;
                if nx < 0 || ny < 0 || nx < xs || nx > xe || ny < ys || ny > ye {
                    continue;
                }
                let idx = self.idx(nx as usize, ny as usize);
                count += self.cells[idx];
            }
        }
        count
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

#[derive(Clone, Debug)]
pub struct StepMetrics {
    pub pop: usize,
    pub flips: usize,
    pub bbox: Option<BBox>,
    pub hash: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_from_points(width: usize, height: usize, points: &[(usize, usize)]) -> Seed {
        Seed {
            width,
            height,
            live_cells: points.to_vec(),
        }
    }

    #[test]
    fn blinker_period_two() {
        let seed = seed_from_points(5, 5, &[(2, 1), (2, 2), (2, 3)]);
        let mut state = LifeState::from_seed(&seed);
        let initial = state.canonical_hash();
        state.step();
        let second = state.step().hash;
        assert_eq!(initial, second);
    }

    #[test]
    fn block_still_life() {
        let seed = seed_from_points(4, 4, &[(1, 1), (1, 2), (2, 1), (2, 2)]);
        let mut state = LifeState::from_seed(&seed);
        let metrics = state.step();
        assert_eq!(metrics.flips, 0);
        assert_eq!(metrics.pop, 4);
    }
}
