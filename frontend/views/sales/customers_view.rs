//! List customers and add new ones.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::parties::customers::Customer;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    customers: Vec<Customer>,
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
    match repo.list_customers_for_company(ctx.company_id) {
        Ok(customers) => State { customers, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::EmailChanged(value) => state.email_input = value,
        Message::Create => {
            let mut customer = Customer::new(ctx.company_id, state.name_input.clone());
            if !state.email_input.trim().is_empty() {
                customer.email = Some(state.email_input.clone());
            }
            if let Err(err) = customer.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let repo = PartyRepository::new(ctx.db.clone());
            match repo.create_customer(&customer) {
                Ok(created) => {
                    state.customers.push(created);
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
    let mut list = column![row![
        text("Name").width(Length::Fill),
        text("Email").width(Length::Fill),
        text("Terms (days)").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for customer in &state.customers {
        list = list.push(row![
            text(customer.name.clone()).width(Length::Fill),
            text(customer.email.clone().unwrap_or_default()).width(Length::Fill),
            text(customer.payment_terms_days.to_string()).width(Length::Fixed(100.0)),
        ]);
    }

    let form = row![
        text_input("Customer name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        text_input("Email (optional)", &state.email_input).on_input(Message::EmailChanged).width(Length::Fill).padding(6),
        button(text("Add customer")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Customers").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
