//! List warehouses and add new ones.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::warehouses::Warehouse;
use crate::backend::services::inventory_service::InventoryService;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    warehouses: Vec<Warehouse>,
    name_input: String,
    code_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    CodeChanged(String),
    Create,
}

pub fn load(ctx: &Ctx) -> State {
    let service = InventoryService::new(ctx.db.clone());
    match service.list_warehouses(ctx.company_id) {
        Ok(warehouses) => State { warehouses, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::CodeChanged(value) => state.code_input = value,
        Message::Create => {
            let warehouse = Warehouse::new(ctx.company_id, state.name_input.clone(), state.code_input.clone());
            if let Err(err) = warehouse.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let service = InventoryService::new(ctx.db.clone());
            match service.create_warehouse(&warehouse) {
                Ok(created) => {
                    state.warehouses.push(created);
                    state.name_input.clear();
                    state.code_input.clear();
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![text("Code").width(Length::Fixed(100.0)), text("Name").width(Length::Fill)]].spacing(4);

    for warehouse in &state.warehouses {
        list = list.push(row![
            text(warehouse.code.clone()).width(Length::Fixed(100.0)),
            text(warehouse.name.clone()).width(Length::Fill),
        ]);
    }

    let form = row![
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fixed(100.0)).padding(6),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        button(text("Add warehouse")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Warehouses").size(24), scrollable(list).height(Length::Fill), form]
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
    use crate::backend::repositories::inventory_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(inventory_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Main Warehouse".into()), &ctx);
        update(&mut state, Message::CodeChanged("MAIN".into()), &ctx);

        assert_eq!(state.name_input, "Main Warehouse");
        assert_eq!(state.code_input, "MAIN");
    }

    #[test]
    fn creating_with_a_blank_name_is_rejected_before_reaching_the_service() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "".into(), code_input: "MAIN".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_some());
        assert!(state.warehouses.is_empty());
    }

    #[test]
    fn creating_a_valid_warehouse_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Main Warehouse".into(), code_input: "MAIN".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.warehouses.len(), 1);
        assert_eq!(state.warehouses[0].code, "MAIN");
        assert!(state.name_input.is_empty());
        assert!(state.code_input.is_empty());
    }
}
