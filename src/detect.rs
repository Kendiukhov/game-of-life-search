use crate::life_core::LifeState;
use ahash::AHashSet;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::hash::Hasher;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetectionConfig {
    pub max_component_cells: usize,
    pub max_components: usize,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            max_component_cells: 128,
            max_components: 256,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DetectorStats {
    pub spaceship_events: usize,
    pub oscillator_events: usize,
}

#[derive(Clone, Debug)]
struct ComponentSignature {
    signature: u64,
    anchor: (i32, i32),
}

#[derive(Clone, Debug)]
struct Snapshot {
    components: Vec<ComponentSignature>,
    step: u64,
}

pub struct Detector {
    config: DetectionConfig,
    last: Option<Snapshot>,
}

impl Detector {
    pub fn new(config: DetectionConfig) -> Self {
        Self {
            config,
            last: None,
        }
    }

    pub fn observe(&mut self, state: &LifeState, step: u64) -> DetectorStats {
        let components = extract_components(state, &self.config);
        let snapshot = Snapshot { components, step };
        let mut stats = DetectorStats::default();

        if let Some(prev) = &self.last {
            for comp in &snapshot.components {
                for p in &prev.components {
                    if comp.signature == p.signature {
                        let dx = comp.anchor.0 - p.anchor.0;
                        let dy = comp.anchor.1 - p.anchor.1;
                        if dx != 0 || dy != 0 {
                            stats.spaceship_events += 1;
                        } else if step > prev.step {
                            stats.oscillator_events += 1;
                        }
                    }
                }
            }
        }

        self.last = Some(snapshot);
        stats
    }
}

fn extract_components(state: &LifeState, config: &DetectionConfig) -> Vec<ComponentSignature> {
    let mut components = Vec::new();
    let bbox = match state.bbox() {
        Some(bb) => bb,
        None => return components,
    };

    let mut visited: AHashSet<(usize, usize)> = AHashSet::new();
    for y in bbox.min_y..=bbox.max_y {
        for x in bbox.min_x..=bbox.max_x {
            if !state.is_alive(x, y) || visited.contains(&(x, y)) {
                continue;
            }
            let comp = flood_fill(state, (x, y), &mut visited, config);
            if let Some(sig) = comp {
                components.push(sig);
                if components.len() >= config.max_components {
                    return components;
                }
            }
        }
    }

    components
}

fn flood_fill(
    state: &LifeState,
    start: (usize, usize),
    visited: &mut AHashSet<(usize, usize)>,
    config: &DetectionConfig,
) -> Option<ComponentSignature> {
    let mut stack = vec![start];
    let mut cells = Vec::new();
    let mut min_x = start.0;
    let mut max_x = start.0;
    let mut min_y = start.1;
    let mut max_y = start.1;

    while let Some((x, y)) = stack.pop() {
        if !visited.insert((x, y)) {
            continue;
        }
        if !state.is_alive(x, y) {
            continue;
        }
        cells.push((x as i32, y as i32));
        min_x = min(min_x, x);
        max_x = max(max_x, x);
        min_y = min(min_y, y);
        max_y = max(max_y, y);
        if cells.len() >= config.max_component_cells {
            break;
        }

        for ny in y.saturating_sub(1)..=min(y + 1, state.height - 1) {
            for nx in x.saturating_sub(1)..=min(x + 1, state.width - 1) {
                if nx == x && ny == y {
                    continue;
                }
                if state.is_alive(nx, ny) && !visited.contains(&(nx, ny)) {
                    stack.push((nx, ny));
                }
            }
        }
    }

    if cells.is_empty() {
        return None;
    }

    let anchor = (min_x as i32, min_y as i32);
    let signature = signature_from_cells(&cells, anchor);
    Some(ComponentSignature {
        signature,
        anchor,
    })
}

fn signature_from_cells(cells: &[(i32, i32)], anchor: (i32, i32)) -> u64 {
    let mut hasher = ahash::AHasher::default();
    for &(x, y) in cells {
        let nx = x - anchor.0;
        let ny = y - anchor.1;
        hasher.write(&nx.to_le_bytes());
        hasher.write(&ny.to_le_bytes());
    }
    hasher.finish()
}
