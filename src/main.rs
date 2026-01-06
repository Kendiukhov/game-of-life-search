use anyhow::Result;
use clap::{Parser, Subcommand};
use lifeminer::{rle_from_seed, ArchiveWriter, LifeState, SearchConfig, SearchRunner};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "lifeminer: Conway's Life pattern mining")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Subcommand)]
enum Commands {
    /// Run MAP-Elites search and persist an archive
    Search {
        #[arg(long, default_value = "archive")]
        out: PathBuf,
        #[arg(long, default_value_t = 48)]
        width: usize,
        #[arg(long, default_value_t = 48)]
        height: usize,
        #[arg(long, default_value_t = 200)]
        iterations: usize,
        #[arg(long, default_value_t = 64)]
        initial_population: usize,
        #[arg(long, default_value_t = 0.24)]
        density: f32,
        #[arg(long, default_value_t = 1337)]
        seed: u64,
        #[arg(long, default_value_t = 512)]
        max_steps: usize,
        #[arg(long, default_value_t = 8)]
        lifespan_bins: usize,
        #[arg(long, default_value_t = 8)]
        mobility_bins: usize,
        #[arg(long, default_value_t = 8)]
        activity_bins: usize,
        #[arg(long)]
        min_score: Option<f64>,
    },
    /// Replay a stored pattern to stdout as ASCII
    Replay {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "archive")]
        archive: PathBuf,
        #[arg(long, default_value_t = 64)]
        steps: usize,
        #[arg(long, default_value_t = 1)]
        stride: usize,
    },
    /// Export a stored pattern as RLE
    Export {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "archive")]
        archive: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Summarize archive stats
    Stats {
        #[arg(long, default_value = "archive")]
        archive: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Search {
            out,
            width,
            height,
            iterations,
            initial_population,
            density,
            seed,
            max_steps,
            lifespan_bins,
            mobility_bins,
            activity_bins,
            min_score,
        } => {
            let mut config = SearchConfig::default();
            config.width = width;
            config.height = height;
            config.iterations = iterations;
            config.initial_population = initial_population;
            config.initial_density = density;
            config.base_seed = seed;
            config.eval.max_steps = max_steps;
            config.map.lifespan_bins = lifespan_bins;
            config.map.mobility_bins = mobility_bins;
            config.map.activity_bins = activity_bins;
            config.min_score = min_score;

            let writer = ArchiveWriter::new(&out)?;
            let mut runner = SearchRunner::new(config);
            let summary = runner.run(Some(&writer))?;
            println!(
                "Search complete: {} iterations, archive {}, accepted {}",
                summary.iterations, summary.archive_size, summary.accepted
            );
        }
        Commands::Replay {
            id,
            archive,
            steps,
            stride,
        } => {
            let writer = ArchiveWriter::new(&archive)?;
            let record = writer
                .load_record(&id)?
                .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", id))?;
            let mut state = LifeState::from_seed(&record.seed);
            println!("Replaying pattern {} ({:?})", id, record.evaluation.outcome);
            for step in 0..steps {
                if step % stride == 0 {
                    println!("Step {}", step);
                    println!("{}", state.to_ascii(2));
                    println!();
                }
                state.step();
            }
        }
        Commands::Export { id, archive, out } => {
            let writer = ArchiveWriter::new(&archive)?;
            let record = writer
                .load_record(&id)?
                .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", id))?;
            let rle = rle_from_seed(&record.seed);
            std::fs::write(&out, rle)?;
            println!("Wrote {}", out.display());
        }
        Commands::Stats { archive } => {
            let writer = ArchiveWriter::new(&archive)?;
            let records = writer.load_all()?;
            if records.is_empty() {
                println!("No records found");
            } else {
                let best = records
                    .iter()
                    .max_by(|a, b| a.evaluation.score.partial_cmp(&b.evaluation.score).unwrap())
                    .unwrap();
                let outcomes = records.iter().fold(HashMap::new(), |mut acc, r| {
                    *acc.entry(format!("{:?}", r.evaluation.outcome))
                        .or_insert(0usize) += 1;
                    acc
                });
                println!("Records: {}", records.len());
                println!("Best score: {:.3} ({})", best.evaluation.score, best.id);
                println!("Outcome breakdown:");
                for (k, v) in outcomes {
                    println!("  {}: {}", k, v);
                }
            }
        }
    }
    Ok(())
}
