//! List vendors and add new ones. Grouped under `inventory/` rather than
//! a dedicated `purchases/` directory -- the frozen frontend/views/
//! structure (Section 2) only has nine subdirectories and none of them
//! is `purchases/`; purchasing pairs naturally with inventory (goods
//! bought feed stock) in the same way it does in most ERP navigation.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::parties::vendors::Vendor;
use crate::backend::services::party_service::PartyService;
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
    let service = PartyService::new(ctx.db.clone());
    match service.list_vendors(ctx.company_id) {
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
            let service = PartyService::new(ctx.db.clone());
            match service.create_vendor(&vendor) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Acme Supplies".into()), &ctx);
        update(&mut state, Message::EmailChanged("ap@acmesupplies.test".into()), &ctx);

        assert_eq!(state.name_input, "Acme Supplies");
        assert_eq!(state.email_input, "ap@acmesupplies.test");
    }

    #[test]
    fn creating_with_a_blank_name_is_rejected_before_reaching_the_service() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_some());
        assert!(state.vendors.is_empty());
    }

    #[test]
    fn creating_with_a_blank_email_leaves_it_unset() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Supplies".into(), email_input: "".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.vendors[0].email, None);
    }

    #[test]
    fn creating_a_valid_vendor_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Supplies".into(), email_input: "ap@acmesupplies.test".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.vendors.len(), 1);
        assert_eq!(state.vendors[0].email.as_deref(), Some("ap@acmesupplies.test"));
        assert!(state.name_input.is_empty());
        assert!(state.email_input.is_empty());
    }
}
