use crate::detect::Detector;
use crate::life_core::{LifeState, Seed};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalConfig {
    pub max_steps: usize,
    pub snapshot_stride: usize,
    pub late_window: usize,
    pub detect_stride: usize,
    pub boundary_stop: bool,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            max_steps: 512,
            snapshot_stride: 1,
            late_window: 32,
            detect_stride: 4,
            boundary_stop: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Outcome {
    Dead,
    Stable,
    Cycle { period: usize },
    Escaped,
    MaxSteps,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Features {
    pub lifespan: usize,
    pub max_pop: usize,
    pub mean_pop: f64,
    pub mean_flips: f64,
    pub late_activity: f64,
    pub max_bbox_diag: f64,
    pub max_bbox_area: usize,
    pub cycle_period: Option<usize>,
    pub spaceship_events: usize,
    pub boundary_touches: usize,
    pub activity_peaks: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
    pub score: f64,
    pub outcome: Outcome,
    pub features: Features,
    pub novelty_vector: Vec<f64>,
}

pub fn evaluate_seed(
    seed: &Seed,
    config: &EvalConfig,
    detector: &mut Option<Detector>,
) -> Evaluation {
    let mut state = LifeState::from_seed(seed);
    let mut seen: AHashMap<u64, usize> = AHashMap::new();
    let mut pop_history = Vec::new();
    let mut flips_history = Vec::new();
    let mut bbox_diag_max = 0.0f64;
    let mut bbox_area_max = 0usize;
    let mut boundary_touches = 0usize;
    let mut spaceship_events = 0usize;
    let mut outcome = Outcome::MaxSteps;

    for step in 0..config.max_steps {
        let metrics = state.step();
        pop_history.push(metrics.pop);
        flips_history.push(metrics.flips);
        if let Some(bb) = metrics.bbox.clone() {
            bbox_diag_max = bbox_diag_max.max(bb.diag());
            bbox_area_max = bbox_area_max.max(bb.area());
            if bb.touches_bounds(state.width, state.height) {
                boundary_touches += 1;
                if config.boundary_stop {
                    outcome = Outcome::Escaped;
                    break;
                }
            }
        }

        if let Some(det) = detector {
            if step % config.detect_stride == 0 {
                let stats = det.observe(&state, step as u64);
                spaceship_events += stats.spaceship_events;
            }
        }

        if metrics.pop == 0 {
            outcome = Outcome::Dead;
            break;
        }
        if metrics.flips == 0 {
            outcome = Outcome::Stable;
            break;
        }
        if let Some(prev) = seen.insert(metrics.hash, step) {
            outcome = Outcome::Cycle {
                period: step - prev,
            };
            break;
        }
    }

    let lifespan = pop_history.len();
    let max_pop = pop_history.iter().copied().max().unwrap_or(0);
    let mean_pop = if lifespan > 0 {
        pop_history.iter().sum::<usize>() as f64 / lifespan as f64
    } else {
        0.0
    };
    let mean_flips = if lifespan > 0 {
        flips_history.iter().sum::<usize>() as f64 / lifespan as f64
    } else {
        0.0
    };

    let late_window = config.late_window.min(flips_history.len());
    let late_activity = if late_window > 0 {
        let tail = &flips_history[flips_history.len() - late_window..];
        tail.iter().sum::<usize>() as f64 / late_window as f64
    } else {
        0.0
    };

    let activity_peaks = count_peaks(&flips_history);
    let cycle_period = match outcome {
        Outcome::Cycle { period } => Some(period),
        _ => None,
    };

    let boundary_penalty = boundary_touches as f64;
    let score = 2.0 * ((lifespan + 1) as f64).ln()
        + 1.5 * (1.0 + late_activity).ln()
        + (1.0 + bbox_diag_max).ln()
        + 4.0 * spaceship_events as f64
        - 3.0 * boundary_penalty
        - 0.5 * seed.density() as f64 * 10.0;

    let features = Features {
        lifespan,
        max_pop,
        mean_pop,
        mean_flips,
        late_activity,
        max_bbox_diag: bbox_diag_max,
        max_bbox_area: bbox_area_max,
        cycle_period,
        spaceship_events,
        boundary_touches,
        activity_peaks,
    };

    let novelty_vector = vec![
        lifespan as f64,
        max_pop as f64,
        late_activity,
        bbox_diag_max,
        cycle_period.unwrap_or(0) as f64,
        spaceship_events as f64,
    ];

    Evaluation {
        score,
        outcome,
        features,
        novelty_vector,
    }
}

fn count_peaks(series: &[usize]) -> usize {
    if series.len() < 3 {
        return 0;
    }
    let mut peaks = 0;
    for window in series.windows(3) {
        if window[1] > window[0] && window[1] > window[2] {
            peaks += 1;
        }
    }
    peaks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::life_core::Seed;

    #[test]
    fn detects_simple_cycle() {
        let seed = Seed {
            width: 5,
            height: 5,
            live_cells: vec![(2, 1), (2, 2), (2, 3)],
        };
        let cfg = EvalConfig {
            max_steps: 10,
            ..Default::default()
        };
        let mut detector = None;
        let eval = evaluate_seed(&seed, &cfg, &mut detector);
        assert!(matches!(eval.outcome, Outcome::Cycle { .. }));
    }
}
