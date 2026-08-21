//! List warehouses and add new ones.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::warehouses::Warehouse;
use crate::backend::repositories::inventory_repository::InventoryRepository;
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
    let repo = InventoryRepository::new(ctx.db.clone());
    match repo.list_warehouses_for_company(ctx.company_id) {
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
            let repo = InventoryRepository::new(ctx.db.clone());
            match repo.create_warehouse(&warehouse) {
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
