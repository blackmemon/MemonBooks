// Application entry point -- Iced application foundation
// (Development Order step 1, PROJECT_SPEC.md Section 5).

use iced::widget::{center, text};
use iced::{Element, Task};

use memonbooks::backend::audit::audit_logs;
use memonbooks::backend::core::logging;
use memonbooks::backend::repositories::{
    accounting_repository, banking_repository, expense_repository, fixed_asset_repository, inventory_repository,
    organization_repository, party_repository, purchase_repository, sales_repository, tax_repository, user_repository,
};
use memonbooks::configs::app::AppConfig;
use memonbooks::database::database::{Database, Migration};
use memonbooks::frontend::styles::{colors, typography};

/// Every module that owns schema contributes its `MIGRATIONS` here, in the
/// order its Development Order step was implemented. `Database::run_migrations`
/// re-sorts by version anyway, so this list only has to be complete, not ordered.
fn all_migrations() -> Vec<Migration> {
    let mut migrations = Vec::new();
    migrations.extend_from_slice(organization_repository::MIGRATIONS);
    migrations.extend_from_slice(user_repository::MIGRATIONS);
    migrations.extend_from_slice(party_repository::MIGRATIONS);
    migrations.extend_from_slice(accounting_repository::MIGRATIONS);
    migrations.extend_from_slice(sales_repository::MIGRATIONS);
    migrations.extend_from_slice(purchase_repository::MIGRATIONS);
    migrations.extend_from_slice(inventory_repository::MIGRATIONS);
    migrations.extend_from_slice(banking_repository::MIGRATIONS);
    migrations.extend_from_slice(expense_repository::MIGRATIONS);
    migrations.extend_from_slice(fixed_asset_repository::MIGRATIONS);
    migrations.extend_from_slice(tax_repository::MIGRATIONS);
    migrations.extend_from_slice(audit_logs::MIGRATIONS);
    migrations
}

fn main() -> iced::Result {
    let config = AppConfig::load().expect("failed to resolve application configuration");
    logging::init(&config).expect("failed to initialize logging");

    log::info!("MemonBooks starting in {} mode", config.environment);
    log::info!("database path: {}", config.database_path.display());

    let database = Database::open(&config.database_path).expect("failed to open database");
    database
        .run_migrations(&all_migrations())
        .expect("failed to run database migrations");

    iced::application("MemonBooks", MemonBooks::update, MemonBooks::view)
        .run_with(move || (MemonBooks { database: database.clone() }, Task::none()))
}

struct MemonBooks {
    #[allow(dead_code)]
    database: Database,
}

#[derive(Debug, Clone, Copy)]
enum Message {}

impl MemonBooks {
    fn update(&mut self, message: Message) {
        match message {}
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            text("MemonBooks")
                .size(typography::BASE_FONT_SIZE * 2.0)
                .color(colors::WHITE_PRIMARY),
        )
        .into()
    }
}
