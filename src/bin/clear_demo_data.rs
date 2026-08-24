//! Admin tool: deletes every `DEMO-`-tagged record from the configured
//! database (the same database `src/main.rs` and `seed_demo_data` use --
//! resolved the same way, via `AppConfig::load`). Never run as part of
//! normal app startup; run explicitly:
//!
//!     cargo run --bin clear_demo_data
//!
//! See `src/demo_data.rs` for what "DEMO-tagged" means and why deleting
//! it is safe to do without touching real data.

use memonbooks::configs::app::AppConfig;
use memonbooks::database::database::Database;
use memonbooks::demo_data::clear_demo_data;

fn main() {
    if let Err(err) = run() {
        eprintln!("clear_demo_data failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> memonbooks::backend::core::errors::AppResult<()> {
    let config = AppConfig::load()?;
    println!("database: {}", config.database_path.display());

    let database = Database::open(&config.database_path)?;
    database.run_migrations(&memonbooks::all_migrations())?;

    let summary = clear_demo_data(&database)?;

    if summary.companies_removed == 0 && summary.users_removed == 0 && summary.roles_removed == 0 {
        println!("No demo data found -- nothing to remove.");
    } else {
        println!(
            "Removed {} demo compan{}, {} demo user{}, {} demo role{} (and everything under them).",
            summary.companies_removed,
            if summary.companies_removed == 1 { "y" } else { "ies" },
            summary.users_removed,
            if summary.users_removed == 1 { "" } else { "s" },
            summary.roles_removed,
            if summary.roles_removed == 1 { "" } else { "s" },
        );
    }

    Ok(())
}
