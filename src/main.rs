// Application entry point -- Iced application foundation
// (Development Order step 1, PROJECT_SPEC.md Section 5).

use iced::widget::{center, text};
use iced::Element;

use memonbooks::frontend::styles::{colors, typography};

fn main() -> iced::Result {
    iced::application("MemonBooks", MemonBooks::update, MemonBooks::view).run()
}

#[derive(Default)]
struct MemonBooks;

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
