use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod analysis;
mod checker;
mod generator;
mod scoring;

/// Auto-generate high-quality READMEs from Rust crate source code.
#[derive(Parser)]
#[command(name = "readme-generator", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a README.md for a crate
    Generate {
        /// Path to the crate root
        #[arg(short, long)]
        crate_path: PathBuf,

        /// Output file (defaults to crate_path/README.md)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show diff without writing
        #[arg(long)]
        diff: bool,

        /// Overwrite existing README without prompting
        #[arg(long)]
        force: bool,
    },
    /// Score an existing README for completeness (0-100)
    Check {
        /// Path to the crate root
        #[arg(short, long)]
        crate_path: PathBuf,

        /// Output scores as JSON
        #[arg(long)]
        json: bool,
    },
    /// Score an existing README and show what would change
    ScoreDiff {
        /// Path to the crate root
        #[arg(short, long)]
        crate_path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            crate_path,
            output,
            diff,
            force,
        } => {
            let crate_info = analysis::analyze_crate(&crate_path)?;
            let readme = generator::generate_readme(&crate_info)?;

            let out_path = output.unwrap_or_else(|| crate_path.join("README.md"));

            if diff {
                if out_path.exists() {
                    let existing = std::fs::read_to_string(&out_path)?;
                    let diff_text = similar::TextDiff::from_lines(&existing, &readme);
                    println!("{}", "=== Diff: existing → generated ===".cyan().bold());
                    for change in diff_text.iter_all_changes() {
                        let sign = match change.tag() {
                            similar::ChangeTag::Delete => "-",
                            similar::ChangeTag::Insert => "+",
                            similar::ChangeTag::Equal => " ",
                        };
                        let colored_line = match change.tag() {
                            similar::ChangeTag::Delete => change.to_string().red(),
                            similar::ChangeTag::Insert => change.to_string().green(),
                            similar::ChangeTag::Equal => change.to_string().dimmed(),
                        };
                        print!("{}{}", sign.bold(), colored_line);
                    }
                } else {
                    println!(
                        "{}",
                        "No existing README found. Would create:".cyan().bold()
                    );
                    println!("{}", readme);
                }
            } else {
                if out_path.exists() && !force {
                    anyhow::bail!(
                        "README already exists at {}. Use --force to overwrite or --diff to preview.",
                        out_path.display()
                    );
                }
                std::fs::write(&out_path, &readme)?;
                println!(
                    "{} README written to {}",
                    "✓".green().bold(),
                    out_path.display()
                );
            }
        }
        Commands::Check { crate_path, json } => {
            let crate_info = analysis::analyze_crate(&crate_path)?;
            let readme_path = crate_path.join("README.md");
            let score_data = if readme_path.exists() {
                let existing = std::fs::read_to_string(&readme_path)?;
                scoring::score_readme(&existing, &crate_info)
            } else {
                scoring::score_readme("", &crate_info)
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&score_data)?);
            } else {
                println!("{}", scoring::format_score_report(&score_data));
            }
        }
        Commands::ScoreDiff { crate_path } => {
            let crate_info = analysis::analyze_crate(&crate_path)?;
            let readme_path = crate_path.join("README.md");

            println!("{}", "=== Current README Score ===".cyan().bold());
            let before_score = if readme_path.exists() {
                let existing = std::fs::read_to_string(&readme_path)?;
                let s = scoring::score_readme(&existing, &crate_info);
                println!("{}", scoring::format_score_report(&s));
                s
            } else {
                let s = scoring::score_readme("", &crate_info);
                println!("(no existing README)");
                println!("{}", scoring::format_score_report(&s));
                s
            };

            let generated = generator::generate_readme(&crate_info)?;
            let after_score = scoring::score_readme(&generated, &crate_info);

            println!();
            println!("{}", "=== Generated README Score ===".cyan().bold());
            println!("{}", scoring::format_score_report(&after_score));

            println!();
            let improvement = after_score.total as i64 - before_score.total as i64;
            if improvement > 0 {
                println!(
                    "{} Score improvement: +{} points ({} → {})",
                    "📈".to_string(),
                    improvement,
                    before_score.total,
                    after_score.total
                );
            } else if improvement < 0 {
                println!(
                    "{} Score change: {} points ({} → {})",
                    "📉",
                    improvement,
                    before_score.total,
                    after_score.total
                );
            } else {
                println!(
                    "{} Score unchanged: {} points",
                    "➡️",
                    after_score.total
                );
            }
        }
    }

    Ok(())
}
