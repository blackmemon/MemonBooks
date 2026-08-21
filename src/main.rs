// Application entry point -- Iced application foundation
// (Development Order step 1, PROJECT_SPEC.md Section 5).

use iced::widget::{center, text};
use iced::{Element, Task};

use memonbooks::backend::core::logging;
use memonbooks::configs::app::AppConfig;
use memonbooks::database::database::Database;
use memonbooks::frontend::styles::{colors, typography};

fn main() -> iced::Result {
    let config = AppConfig::load().expect("failed to resolve application configuration");
    logging::init(&config).expect("failed to initialize logging");

    log::info!("MemonBooks starting in {} mode", config.environment);
    log::info!("database path: {}", config.database_path.display());

    let database = Database::open(&config.database_path).expect("failed to open database");
    database
        .run_migrations(&[])
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
