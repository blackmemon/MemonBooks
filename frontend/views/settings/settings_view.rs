//! Company settings: view and edit the current company's basic info.

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::Element;

use crate::backend::core::validation::Validate;
use crate::backend::organization::companies::Company;
use crate::backend::services::organization_service::OrganizationService;
use crate::frontend::styles::button as button_style;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::font_scale::{self, FontScale};
use crate::frontend::styles::inputs;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    company: Option<Company>,
    name_input: String,
    legal_name_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    LegalNameChanged(String),
    ThemeChanged(Theme),
    FontScaleChanged(FontScale),
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    None,
    ThemeChanged(Theme),
    FontScaleChanged(FontScale),
}

pub fn load(ctx: &Ctx) -> State {
    let service = OrganizationService::new(ctx.db.clone());
    match service.get_company(ctx.company_id) {
        Ok(company) => State {
            name_input: company.name.clone(),
            legal_name_input: company.legal_name.clone().unwrap_or_default(),
            company: Some(company),
            message: None,
            error: None,
        },
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) -> Event {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::LegalNameChanged(value) => state.legal_name_input = value,
        Message::ThemeChanged(theme) => return Event::ThemeChanged(theme),
        Message::FontScaleChanged(scale) => return Event::FontScaleChanged(scale),
        Message::Save => {
            let Some(company) = state.company.clone() else {
                return Event::None;
            };
            let updated = Company {
                name: state.name_input.clone(),
                legal_name: if state.legal_name_input.trim().is_empty() {
                    None
                } else {
                    Some(state.legal_name_input.clone())
                },
                ..company
            };
            if let Err(err) = updated.validate() {
                state.error = Some(err.to_string());
                state.message = None;
                return Event::None;
            }
            let service = OrganizationService::new(ctx.db.clone());
            match service.update_company(&updated) {
                Ok(()) => {
                    state.company = Some(updated);
                    state.message = Some("Saved.".to_string());
                    state.error = None;
                }
                Err(err) => {
                    state.error = Some(err.to_string());
                    state.message = None;
                }
            }
        }
    }
    Event::None
}

pub fn view(state: &State, theme: Theme, font_scale: FontScale) -> Element<'_, Message> {
    let Some(company) = &state.company else {
        let message = state
            .error
            .clone()
            .unwrap_or_else(|| "Loading...".to_string());
        return container(text(message).color(theme.negative_front()))
            .padding(spacing::SPACE_20)
            .into();
    };

    let light_theme_button: Element<'_, Message> = button(text("Light"))
        .style(button_style::mode(theme, theme == Theme::Light))
        .on_press(Message::ThemeChanged(Theme::Light))
        .padding(spacing::SM)
        .into();
    let dark_theme_button: Element<'_, Message> = button(text("Dark"))
        .style(button_style::mode(theme, theme == Theme::Dark))
        .on_press(Message::ThemeChanged(Theme::Dark))
        .padding(spacing::SM)
        .into();

    let font_size_picker: Element<'_, Message> =
        pick_list(font_scale::ALL, Some(font_scale), Message::FontScaleChanged)
            .style(dropdown::style(theme))
            .menu_style(dropdown::dropdown_menu(theme))
            .padding(spacing::SM)
            .width(iced::Length::Fixed(180.0))
            .into();

    let mut content = column![
        text("Company Settings").size(typography::title()),
        text(format!("Base currency: {}", company.base_currency_code)).size(typography::emphasis()),
        text("Name"),
        text_input("Company name", &state.name_input)
            .style(inputs::text(theme))
            .on_input(Message::NameChanged)
            .padding(spacing::SM),
        text("Legal name"),
        text_input("Legal name (optional)", &state.legal_name_input)
            .style(inputs::text(theme))
            .on_input(Message::LegalNameChanged)
            .padding(spacing::SM),
        text("Appearance").size(typography::label()),
        row![light_theme_button, dark_theme_button].spacing(spacing::SM),
        text("Font Size").size(typography::label()),
        font_size_picker,
        button(text("Save"))
            .style(button_style::primary(theme))
            .on_press(Message::Save)
            .padding(spacing::SM),
    ]
    .spacing(spacing::SPACE_12)
    .padding(spacing::SPACE_20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    content.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        }
    }

    #[test]
    fn load_populates_the_form_from_the_stored_company() {
        let ctx = ctx_with_company();

        let state = load(&ctx);

        assert_eq!(state.name_input, "Acme Trading Co");
        assert!(state.company.is_some());
        assert!(state.error.is_none());
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);

        update(
            &mut state,
            Message::NameChanged("Acme Holdings".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::LegalNameChanged("Acme Holdings LLC".into()),
            &ctx,
        );

        assert_eq!(state.name_input, "Acme Holdings");
        assert_eq!(state.legal_name_input, "Acme Holdings LLC");
    }

    #[test]
    fn saving_with_a_blank_name_is_rejected_and_leaves_the_stored_company_untouched() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        state.name_input = "".into();

        update(&mut state, Message::Save, &ctx);

        assert!(state.error.is_some());
        assert!(state.message.is_none());
        let stored = OrganizationService::new(ctx.db.clone())
            .get_company(ctx.company_id)
            .unwrap();
        assert_eq!(stored.name, "Acme Trading Co");
    }

    #[test]
    fn saving_a_valid_change_persists_it_and_sets_the_message() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        state.name_input = "Acme Holdings".into();
        state.legal_name_input = "Acme Holdings LLC".into();

        update(&mut state, Message::Save, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.message.as_deref(), Some("Saved."));
        assert_eq!(state.company.as_ref().unwrap().name, "Acme Holdings");

        let stored = OrganizationService::new(ctx.db.clone())
            .get_company(ctx.company_id)
            .unwrap();
        assert_eq!(stored.name, "Acme Holdings");
        assert_eq!(stored.legal_name.as_deref(), Some("Acme Holdings LLC"));
    }

    // `update()`-only tests never call `view()`, so they can't catch the
    // iced widget-construction panics that only fire when the widget
    // tree is actually built (e.g. `Scrollable`'s Fill-on-scroll-axis
    // `debug_assert`) -- see the `data_table` precedent for this
    // pattern. Sweeping every `FontScale` here doubles as the
    // construction check for the new Font Size picker this view added.
    #[test]
    fn view_builds_without_panicking_at_every_font_scale() {
        let ctx = ctx_with_company();
        let state = load(&ctx);

        for scale in font_scale::ALL {
            let _: Element<'_, Message> = view(&state, Theme::Light, scale);
        }
    }
}
