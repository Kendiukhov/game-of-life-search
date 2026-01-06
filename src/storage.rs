use crate::eval::Evaluation;
use crate::life_core::Seed;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub rng_seed: u64,
    pub seed: Seed,
    pub evaluation: Evaluation,
    pub created_at: u64,
}

impl PatternRecord {
    pub fn new(id: String, seed: Seed, evaluation: Evaluation, parent_id: Option<String>, rng_seed: u64) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id,
            parent_id,
            rng_seed,
            seed,
            evaluation,
            created_at,
        }
    }
}

pub struct ArchiveWriter {
    records_path: PathBuf,
    pattern_dir: PathBuf,
}

impl ArchiveWriter {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let pattern_dir = root.join("patterns");
        fs::create_dir_all(&pattern_dir)?;
        let records_path = root.join("records.jsonl");
        if !records_path.exists() {
            File::create(&records_path)?;
        }
        Ok(Self {
            records_path,
            pattern_dir,
        })
    }

    pub fn persist(&self, record: &PatternRecord) -> Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.records_path)?;
        let json = serde_json::to_string(record)?;
        writeln!(file, "{}", json)?;

        let rle = rle_from_seed(&record.seed);
        let rle_path = self.pattern_dir.join(format!("{}.rle", record.id));
        fs::write(rle_path, rle)?;
        Ok(())
    }

    pub fn load_record(&self, id: &str) -> Result<Option<PatternRecord>> {
        let file = File::open(&self.records_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let rec: PatternRecord = serde_json::from_str(&line)?;
            if rec.id == id {
                return Ok(Some(rec));
            }
        }
        Ok(None)
    }

    pub fn load_all(&self) -> Result<Vec<PatternRecord>> {
        let file = File::open(&self.records_path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let rec: PatternRecord = serde_json::from_str(&line)?;
            records.push(rec);
        }
        Ok(records)
    }
}

pub fn rle_from_seed(seed: &Seed) -> String {
    if seed.live_cells.is_empty() {
        return "x=0,y=0\n!".to_string();
    }
    let min_x = seed.live_cells.iter().map(|(x, _)| *x).min().unwrap_or(0);
    let max_x = seed.live_cells.iter().map(|(x, _)| *x).max().unwrap_or(0);
    let min_y = seed.live_cells.iter().map(|(_, y)| *y).min().unwrap_or(0);
    let max_y = seed.live_cells.iter().map(|(_, y)| *y).max().unwrap_or(0);

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let mut rows = vec![vec!['b'; width]; height];
    for &(x, y) in &seed.live_cells {
        let rx = x - min_x;
        let ry = y - min_y;
        if ry < rows.len() && rx < rows[ry].len() {
            rows[ry][rx] = 'o';
        }
    }

    let mut body = String::new();
    for (i, row) in rows.iter().enumerate() {
        let mut current = row[0];
        let mut count = 1usize;
        for &cell in row.iter().skip(1) {
            if cell == current {
                count += 1;
            } else {
                push_run(&mut body, count, current);
                current = cell;
                count = 1;
            }
        }
        push_run(&mut body, count, current);
        if i < rows.len() - 1 {
            body.push('$');
        }
    }
    body.push('!');
    format!("x={},y={}\n{}", width, height, body)
}

fn push_run(buf: &mut String, count: usize, cell: char) {
    if count > 1 {
        buf.push_str(&count.to_string());
    }
    buf.push(cell);
}
