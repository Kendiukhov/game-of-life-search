use crate::detect::DetectionConfig;
use crate::eval::{evaluate_seed, EvalConfig, Evaluation};
use crate::life_core::Seed;
use crate::mutate::{CompositeMutator, MutateConfig, Mutator};
use crate::storage::{ArchiveWriter, PatternRecord};
use rand::seq::IteratorRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapElitesConfig {
    pub lifespan_bins: usize,
    pub mobility_bins: usize,
    pub activity_bins: usize,
}

impl Default for MapElitesConfig {
    fn default() -> Self {
        Self {
            lifespan_bins: 8,
            mobility_bins: 8,
            activity_bins: 8,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchConfig {
    pub width: usize,
    pub height: usize,
    pub initial_population: usize,
    pub iterations: usize,
    pub base_seed: u64,
    pub initial_density: f32,
    pub min_score: Option<f64>,
    pub eval: EvalConfig,
    pub map: MapElitesConfig,
    pub mutation: MutateConfig,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            width: 48,
            height: 48,
            initial_population: 64,
            iterations: 200,
            base_seed: 1337,
            initial_density: 0.25,
            min_score: None,
            eval: EvalConfig::default(),
            map: MapElitesConfig::default(),
            mutation: MutateConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Descriptor {
    pub lifespan_bin: u8,
    pub mobility_bin: u8,
    pub activity_bin: u8,
}

#[derive(Default)]
pub struct MapElitesArchive {
    pub map: HashMap<Descriptor, PatternRecord>,
    config: MapElitesConfig,
}

impl MapElitesArchive {
    pub fn new(config: MapElitesConfig) -> Self {
        Self {
            config,
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, record: PatternRecord) -> bool {
        let desc = descriptor_from_features(&record.evaluation, &self.config);
        match self.map.get(&desc) {
            Some(existing) => {
                if record.evaluation.score > existing.evaluation.score {
                    self.map.insert(desc, record);
                    true
                } else {
                    false
                }
            }
            None => {
                self.map.insert(desc, record);
                true
            }
        }
    }

    pub fn random<'a, R: Rng>(&'a self, rng: &mut R) -> Option<&'a PatternRecord> {
        self.map.values().choose(rng)
    }

    pub fn size(&self) -> usize {
        self.map.len()
    }
}

fn descriptor_from_features(eval: &Evaluation, cfg: &MapElitesConfig) -> Descriptor {
    let lifespan_bin = bucket(eval.features.lifespan as f64, cfg.lifespan_bins);
    let mobility_bin = bucket(eval.features.max_bbox_diag, cfg.mobility_bins);
    let activity_bin = bucket(eval.features.late_activity, cfg.activity_bins);
    Descriptor {
        lifespan_bin,
        mobility_bin,
        activity_bin,
    }
}

fn bucket(value: f64, bins: usize) -> u8 {
    if bins == 0 {
        return 0;
    }
    let log = (value + 1.0).log2().floor().max(0.0) as usize;
    let clamped = log.min(bins.saturating_sub(1));
    clamped as u8
}

#[derive(Default)]
pub struct NoveltyArchive {
    pub vectors: Vec<Vec<f64>>,
    pub k: usize,
}

impl NoveltyArchive {
    pub fn new(k: usize) -> Self {
        Self {
            vectors: Vec::new(),
            k,
        }
    }

    pub fn score(&self, v: &[f64]) -> f64 {
        if self.vectors.is_empty() {
            return 0.0;
        }
        let mut distances: Vec<f64> = self.vectors.iter().map(|o| euclidean(v, o)).collect();
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let k = distances.len().min(self.k.max(1));
        distances.into_iter().take(k).sum::<f64>() / k as f64
    }

    pub fn add(&mut self, v: Vec<f64>) {
        self.vectors.push(v);
    }
}

fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

pub struct SearchRunner {
    pub config: SearchConfig,
    pub archive: MapElitesArchive,
    mutator: CompositeMutator,
    novelty: NoveltyArchive,
    rng: ChaCha8Rng,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSummary {
    pub iterations: usize,
    pub archive_size: usize,
    pub accepted: usize,
}

impl SearchRunner {
    pub fn new(config: SearchConfig) -> Self {
        let mutator = CompositeMutator::new(config.mutation.clone());
        let archive = MapElitesArchive::new(config.map.clone());
        Self {
            config,
            archive,
            mutator,
            novelty: NoveltyArchive::new(10),
            rng: ChaCha8Rng::seed_from_u64(42),
        }
    }

    pub fn run(&mut self, writer: Option<&ArchiveWriter>) -> anyhow::Result<SearchSummary> {
        self.rng = ChaCha8Rng::seed_from_u64(self.config.base_seed);
        let mut accepted = 0usize;

        let initial_seeds: Vec<Seed> = (0..self.config.initial_population)
            .map(|i| {
                let mut rng = ChaCha8Rng::seed_from_u64(self.config.base_seed + i as u64);
                Seed::random(
                    self.config.width,
                    self.config.height,
                    self.config.initial_density,
                    &mut rng,
                )
            })
            .collect();

        let eval_cfg = self.config.eval.clone();
        let initial_records: Vec<PatternRecord> = initial_seeds
            .into_par_iter()
            .enumerate()
            .map(|(i, seed)| {
                let mut det = Some(crate::detect::Detector::new(DetectionConfig::default()));
                let mut eval = evaluate_seed(&seed, &eval_cfg, &mut det);
                let novelty = 0.0;
                eval.score += novelty;
                let mut rng = ChaCha8Rng::seed_from_u64(eval.features.lifespan as u64 + i as u64);
                let id = format!("{:016x}", rng.gen::<u64>());
                PatternRecord::new(id, seed, eval, None, rng.gen::<u64>())
            })
            .collect();

        for record in initial_records {
            if let Some(min_score) = self.config.min_score {
                if record.evaluation.score < min_score {
                    continue;
                }
            }
            if self.archive.insert(record.clone()) {
                accepted += 1;
                self.novelty.add(record.evaluation.novelty_vector.clone());
                if let Some(w) = writer {
                    w.persist(&record)?;
                }
            }
        }

        for _ in 0..self.config.iterations {
            let (parent_seed, parent_id) = if let Some(parent) = self.archive.random(&mut self.rng)
            {
                (parent.seed.clone(), Some(parent.id.clone()))
            } else {
                (
                    Seed::random(
                        self.config.width,
                        self.config.height,
                        self.config.initial_density,
                        &mut self.rng,
                    ),
                    None,
                )
            };

            let child_seed = self.mutator.mutate(&parent_seed, &mut self.rng);
            let mut det = Some(crate::detect::Detector::new(DetectionConfig::default()));
            let mut eval = evaluate_seed(&child_seed, &self.config.eval, &mut det);
            let novelty = self.novelty.score(&eval.novelty_vector);
            eval.score += novelty * 0.25;
            self.novelty.add(eval.novelty_vector.clone());

            let id = format!("{:016x}", self.rng.gen::<u64>());
            let record = PatternRecord::new(id, child_seed, eval, parent_id, self.rng.gen::<u64>());

            if let Some(min_score) = self.config.min_score {
                if record.evaluation.score < min_score {
                    continue;
                }
            }

            if self.archive.insert(record.clone()) {
                accepted += 1;
                if let Some(w) = writer {
                    w.persist(&record)?;
                }
            }
        }

        Ok(SearchSummary {
            iterations: self.config.iterations,
            archive_size: self.archive.size(),
            accepted,
        })
    }
}
