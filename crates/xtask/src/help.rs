/// Prints CLI usage for available centralized tasks.
pub(crate) fn print_help() {
    println!("tardigrade-xtask");
    println!();
    println!("Usage:");
    println!("  cargo xtask dashboard [install|lint|build|dev|all]");
    println!();
    println!("Examples:");
    println!("  cargo xtask dashboard build");
    println!("  cargo dashboard-build");
}
