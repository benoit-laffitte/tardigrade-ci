use anyhow::{Result, anyhow};
use std::env;
mod dashboard_task;
mod help;
mod task_context;

use dashboard_task::run_dashboard_task;
use help::print_help;
use task_context::resolve_context;

/// Parses CLI arguments and dispatches to requested task.
fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let context = resolve_context()?;

    match args.as_slice() {
        [scope] if scope == "help" || scope == "--help" || scope == "-h" => {
            print_help();
            Ok(())
        }
        [scope] if scope == "dashboard" => run_dashboard_task(&context, "all"),
        [scope, action] if scope == "dashboard" => run_dashboard_task(&context, action),
        _ => {
            print_help();
            Err(anyhow!("invalid task arguments"))
        }
    }
}
