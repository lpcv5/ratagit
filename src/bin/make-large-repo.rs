#[path = "../perf_repo.rs"]
#[allow(dead_code)]
mod perf_repo;

use std::env;
use std::error::Error;

use perf_repo::{LargeRepoConfig, generate_repo, make_large_repo_usage, print_large_repo_summary};

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let config = LargeRepoConfig::parse(args)?;
    if config.help {
        println!("{}", make_large_repo_usage());
        return Ok(());
    }

    let manifest = generate_repo(&config)?;
    print_large_repo_summary(&config, &manifest);
    Ok(())
}
