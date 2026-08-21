//! List products and add new ones.

use rust_decimal::Decimal;

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::products::Product;
use crate::backend::repositories::inventory_repository::InventoryRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    products: Vec<Product>,
    sku_input: String,
    name_input: String,
    price_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SkuChanged(String),
    NameChanged(String),
    PriceChanged(String),
    Create,
}

pub fn load(ctx: &Ctx) -> State {
    let repo = InventoryRepository::new(ctx.db.clone());
    match repo.list_products_for_company(ctx.company_id) {
        Ok(products) => State { products, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::SkuChanged(value) => state.sku_input = value,
        Message::NameChanged(value) => state.name_input = value,
        Message::PriceChanged(value) => state.price_input = value,
        Message::Create => {
            let mut product = Product::new(ctx.company_id, state.sku_input.clone(), state.name_input.clone());
            if !state.price_input.trim().is_empty() {
                match state.price_input.parse::<Decimal>() {
                    Ok(price) => product.sale_price = price,
                    Err(_) => {
                        state.error = Some("sale price is not a valid number".into());
                        return;
                    }
                }
            }
            if let Err(err) = product.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let repo = InventoryRepository::new(ctx.db.clone());
            match repo.create_product(&product) {
                Ok(created) => {
                    state.products.push(created);
                    state.sku_input.clear();
                    state.name_input.clear();
                    state.price_input.clear();
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("SKU").width(Length::Fixed(100.0)),
        text("Name").width(Length::Fill),
        text("Sale Price").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for product in &state.products {
        list = list.push(row![
            text(product.sku.clone()).width(Length::Fixed(100.0)),
            text(product.name.clone()).width(Length::Fill),
            text(product.sale_price.to_string()).width(Length::Fixed(100.0)),
        ]);
    }

    let form = row![
        text_input("SKU", &state.sku_input).on_input(Message::SkuChanged).width(Length::Fixed(120.0)).padding(6),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        text_input("Sale price", &state.price_input).on_input(Message::PriceChanged).width(Length::Fixed(100.0)).padding(6),
        button(text("Add product")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Products").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
