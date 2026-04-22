mod agent;
mod cli;
mod error;
mod launcher;
mod prompts;

use clap::Parser;

use crate::agent::generate_agent_content;
use crate::cli::Cli;
use crate::error::AppError;
use crate::launcher::{check_opencode_in_path, launch_opencode, write_agent_file};
use crate::prompts::ArchitectType;

// main() and print_list() are excluded from coverage: they are entry-point /
// UI functions that orchestrate already-tested building blocks and require a
// live opencode binary to exercise fully.
#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), AppError> {
    let cli = Cli::parse();

    if cli.list {
        print_list();
        return Ok(());
    }

    // architect is guaranteed to be Some when --list is not present (enforced
    // by clap's required_unless_present).
    let architect = cli
        .architect
        .expect("architect is required when --list is not set");

    let model = cli
        .model
        .as_deref()
        .unwrap_or_else(|| architect.default_model());

    let content = generate_agent_content(architect, cli.full, model);

    if cli.dry_run {
        print!("{content}");
        return Ok(());
    }

    check_opencode_in_path()?;

    let cwd = std::env::current_dir().map_err(AppError::CurrentDir)?;
    let path = write_agent_file(&cwd, architect, &content)?;
    eprintln!("Wrote agent file: {}", path.display());
    eprintln!("Tip: add .opencode/agents/arch-*.md to your .gitignore");

    launch_opencode(architect.agent_name())
}

#[cfg(not(tarpaulin_include))]
fn print_list() {
    println!("Available architect prompts:\n");
    for architect in ArchitectType::all() {
        println!(
            "  {:12}  {:45}  {}",
            architect.agent_name().trim_start_matches("arch-"),
            architect.description(),
            architect.default_model(),
        );
    }
}
