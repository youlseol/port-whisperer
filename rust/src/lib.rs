pub mod cli;
pub mod collector;
pub mod display;
pub mod enricher;
pub mod platform;
pub mod runner;
pub mod types;

use anyhow::Result;
use cli::Cli;

pub fn run(cli: Cli) -> Result<()> {
    runner::run(cli)
}
