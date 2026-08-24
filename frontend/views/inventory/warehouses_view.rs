//! List warehouses and add new ones.

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::warehouses::Warehouse;
use crate::backend::services::inventory_service::InventoryService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    warehouses: Vec<Warehouse>,
    name_input: String,
    code_input: String,
    search: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    CodeChanged(String),
    Create,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
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
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, and "Filters" / "Sort by" / the
        // per-row kebab menu are rendered by the shared toolbar/data
        // table components now (Section 6) -- wiring real tab/filter/
        // sort/action behavior is follow-up work, not part of this
        // layout pass.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked | Message::RowAction(_) => {}
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let header = page_header::view("Warehouses", "Every stock location on this company's books.", "Add warehouse", Message::Create);

    let toolbar = toolbar::view(
        &["All Warehouses"],
        0,
        Message::TabSelected,
        &state.search,
        "Search warehouses",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view("Warehouse list", "Every warehouse on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Warehouse> =
        state.warehouses.iter().filter(|warehouse| query.is_empty() || warehouse.name.to_lowercase().contains(&query) || warehouse.code.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Warehouse", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Code", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|warehouse| {
            Row::new(vec![
                Cell::text(warehouse.name.clone()),
                Cell::text(warehouse.code.clone()),
                if warehouse.is_active { Cell::status("Active", BadgeTone::Positive) } else { Cell::status("Inactive", BadgeTone::Neutral) },
            ])
        })
        .collect();

    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), Message::RowAction);

    let form = column![
        text("Add a warehouse").size(13),
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        button(text("Add warehouse")).on_press(Message::Create).padding(8),
    ]
    .spacing(8);

    let mut content = column![header, toolbar, heading, table, form].spacing(20).padding(24).width(Length::Fill);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(scrollable(content).height(Length::Fill)).width(Length::Fill).into()
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

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("main".into()), &ctx);

        assert_eq!(state.search, "main");
    }
}
