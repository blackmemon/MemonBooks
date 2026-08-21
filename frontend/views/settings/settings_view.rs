//! Company settings: view and edit the current company's basic info.

use iced::widget::{button, column, container, text, text_input};
use iced::Element;

use crate::backend::core::validation::Validate;
use crate::backend::organization::companies::Company;
use crate::backend::repositories::organization_repository::OrganizationRepository;
use crate::frontend::styles::colors;
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
    Save,
}

pub fn load(ctx: &Ctx) -> State {
    let repo = OrganizationRepository::new(ctx.db.clone());
    match repo.get_company(ctx.company_id) {
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

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::LegalNameChanged(value) => state.legal_name_input = value,
        Message::Save => {
            let Some(company) = state.company.clone() else { return };
            let updated = Company {
                name: state.name_input.clone(),
                legal_name: if state.legal_name_input.trim().is_empty() { None } else { Some(state.legal_name_input.clone()) },
                ..company
            };
            if let Err(err) = updated.validate() {
                state.error = Some(err.to_string());
                state.message = None;
                return;
            }
            let repo = OrganizationRepository::new(ctx.db.clone());
            match repo.update_company(&updated) {
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
}

pub fn view(state: &State) -> Element<'_, Message> {
    let Some(company) = &state.company else {
        let message = state.error.clone().unwrap_or_else(|| "Loading...".to_string());
        return container(text(message).color(colors::WHITE_NEGATIVE_FRONT)).padding(20).into();
    };

    let mut content = column![
        text("Company Settings").size(24),
        text(format!("Base currency: {}", company.base_currency_code)).size(14),
        text("Name"),
        text_input("Company name", &state.name_input).on_input(Message::NameChanged).padding(8),
        text("Legal name"),
        text_input("Legal name (optional)", &state.legal_name_input).on_input(Message::LegalNameChanged).padding(8),
        button(text("Save")).on_press(Message::Save).padding(8),
    ]
    .spacing(12)
    .padding(20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    content.into()
}
