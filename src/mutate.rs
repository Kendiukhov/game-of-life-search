use crate::life_core::{BBox, Seed};
use ahash::AHashSet;
use rand::seq::IteratorRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutateConfig {
    pub flip_radius: usize,
    pub flip_count: usize,
    pub patch_size: usize,
    pub min_density: f32,
    pub max_density: f32,
}

impl Default for MutateConfig {
    fn default() -> Self {
        Self {
            flip_radius: 2,
            flip_count: 12,
            patch_size: 6,
            min_density: 0.12,
            max_density: 0.35,
        }
    }
}

pub trait Mutator {
    fn mutate<R: Rng>(&self, parent: &Seed, rng: &mut R) -> Seed;
}

pub struct CompositeMutator {
    config: MutateConfig,
}

impl CompositeMutator {
    pub fn new(config: MutateConfig) -> Self {
        Self { config }
    }
}

impl Mutator for CompositeMutator {
    fn mutate<R: Rng>(&self, parent: &Seed, rng: &mut R) -> Seed {
        let mut cells: AHashSet<(usize, usize)> = parent.live_cells.iter().copied().collect();
        match rng.gen_range(0..4) {
            0 => local_flips(&mut cells, parent, &self.config, rng),
            1 => boundary_mutation(&mut cells, parent, &self.config, rng),
            2 => patch_stamping(&mut cells, parent, &self.config, rng),
            _ => density_nudge(&mut cells, parent, &self.config, rng),
        }

        Seed {
            width: parent.width,
            height: parent.height,
            live_cells: cells.into_iter().collect(),
        }
    }
}

fn local_flips<R: Rng>(
    cells: &mut AHashSet<(usize, usize)>,
    parent: &Seed,
    config: &MutateConfig,
    rng: &mut R,
) {
    let anchor = cells
        .iter()
        .copied()
        .choose(rng)
        .unwrap_or((rng.gen_range(0..parent.width), rng.gen_range(0..parent.height)));
    for _ in 0..config.flip_count {
        let dx = rng.gen_range(0..=config.flip_radius) as isize;
        let dy = rng.gen_range(0..=config.flip_radius) as isize;
        let x = clamp_to_grid(anchor.0 as isize + dx - config.flip_radius as isize / 2, parent.width);
        let y = clamp_to_grid(anchor.1 as isize + dy - config.flip_radius as isize / 2, parent.height);
        toggle_cell(cells, x, y);
    }
}

fn boundary_mutation<R: Rng>(
    cells: &mut AHashSet<(usize, usize)>,
    parent: &Seed,
    config: &MutateConfig,
    rng: &mut R,
) {
    let bbox = bbox_for_cells(cells, parent.width, parent.height);
    let (min_x, max_x, min_y, max_y) = match bbox {
        Some(bb) => (bb.min_x, bb.max_x, bb.min_y, bb.max_y),
        None => (0, parent.width - 1, 0, parent.height - 1),
    };
    for _ in 0..config.flip_count {
        let edge = rng.gen_range(0..4);
        let (x, y) = match edge {
            0 => (rng.gen_range(min_x..=max_x), min_y),
            1 => (rng.gen_range(min_x..=max_x), max_y),
            2 => (min_x, rng.gen_range(min_y..=max_y)),
            _ => (max_x, rng.gen_range(min_y..=max_y)),
        };
        let nx = clamp_range(x as isize + rng.gen_range(-1..=1), parent.width);
        let ny = clamp_range(y as isize + rng.gen_range(-1..=1), parent.height);
        toggle_cell(cells, nx, ny);
    }
}

fn patch_stamping<R: Rng>(
    cells: &mut AHashSet<(usize, usize)>,
    parent: &Seed,
    config: &MutateConfig,
    rng: &mut R,
) {
    let max_dim = min(config.patch_size.max(1), min(parent.width, parent.height));
    if max_dim == 0 {
        return;
    }
    let size = rng.gen_range(1..=max_dim);
    let src_x = rng.gen_range(0..parent.width.saturating_sub(size) + 1);
    let src_y = rng.gen_range(0..parent.height.saturating_sub(size) + 1);
    let dst_x = rng.gen_range(0..parent.width.saturating_sub(size) + 1);
    let dst_y = rng.gen_range(0..parent.height.saturating_sub(size) + 1);

    for y in 0..size {
        for x in 0..size {
            let src_coord = (src_x + x, src_y + y);
            if cells.contains(&src_coord) {
                let dest_coord = (dst_x + x, dst_y + y);
                toggle_cell(cells, dest_coord.0, dest_coord.1);
            }
        }
    }
}

fn density_nudge<R: Rng>(
    cells: &mut AHashSet<(usize, usize)>,
    parent: &Seed,
    config: &MutateConfig,
    rng: &mut R,
) {
    if parent.width == 0 || parent.height == 0 {
        return;
    }
    let max_cells = (parent.width * parent.height) as f32;
    let density = cells.len() as f32 / max_cells;
    if density > config.max_density {
        let remove_count = ((density - config.max_density) * max_cells) as usize;
        for _ in 0..remove_count.min(cells.len()) {
            if let Some(cell) = cells.iter().copied().choose(rng) {
                cells.remove(&cell);
            }
        }
    } else if density < config.min_density {
        let add_count = ((config.min_density - density) * max_cells) as usize;
        for _ in 0..add_count {
            let x = rng.gen_range(0..parent.width);
            let y = rng.gen_range(0..parent.height);
            cells.insert((x, y));
        }
    }
}

fn bbox_for_cells(
    cells: &AHashSet<(usize, usize)>,
    width: usize,
    height: usize,
) -> Option<BBox> {
    if cells.is_empty() {
        return None;
    }
    let mut min_x = width - 1;
    let mut max_x = 0usize;
    let mut min_y = height - 1;
    let mut max_y = 0usize;
    for &(x, y) in cells {
        min_x = min(min_x, x);
        max_x = max(max_x, x);
        min_y = min(min_y, y);
        max_y = max(max_y, y);
    }
    Some(BBox {
        min_x,
        max_x,
        min_y,
        max_y,
    })
}

fn toggle_cell(cells: &mut AHashSet<(usize, usize)>, x: usize, y: usize) {
    if !cells.insert((x, y)) {
        cells.remove(&(x, y));
    }
}

fn clamp_to_grid(value: isize, max_val: usize) -> usize {
    clamp_range(value, max_val)
}

fn clamp_range(value: isize, max_val: usize) -> usize {
    min(max(value, 0) as usize, max_val.saturating_sub(1))
}
