//! List vendors and add new ones. Grouped under `inventory/` rather than
//! a dedicated `purchases/` directory -- the frozen frontend/views/
//! structure (Section 2) only has nine subdirectories and none of them
//! is `purchases/`; purchasing pairs naturally with inventory (goods
//! bought feed stock) in the same way it does in most ERP navigation.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::parties::vendors::Vendor;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    vendors: Vec<Vendor>,
    name_input: String,
    email_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    EmailChanged(String),
    Create,
}

pub fn load(ctx: &Ctx) -> State {
    let repo = PartyRepository::new(ctx.db.clone());
    match repo.list_vendors_for_company(ctx.company_id) {
        Ok(vendors) => State { vendors, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::EmailChanged(value) => state.email_input = value,
        Message::Create => {
            let mut vendor = Vendor::new(ctx.company_id, state.name_input.clone());
            if !state.email_input.trim().is_empty() {
                vendor.email = Some(state.email_input.clone());
            }
            if let Err(err) = vendor.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let repo = PartyRepository::new(ctx.db.clone());
            match repo.create_vendor(&vendor) {
                Ok(created) => {
                    state.vendors.push(created);
                    state.name_input.clear();
                    state.email_input.clear();
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![text("Name").width(Length::Fill), text("Email").width(Length::Fill)]].spacing(4);

    for vendor in &state.vendors {
        list = list.push(row![
            text(vendor.name.clone()).width(Length::Fill),
            text(vendor.email.clone().unwrap_or_default()).width(Length::Fill),
        ]);
    }

    let form = row![
        text_input("Vendor name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        text_input("Email (optional)", &state.email_input).on_input(Message::EmailChanged).width(Length::Fill).padding(6),
        button(text("Add vendor")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Vendors").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
