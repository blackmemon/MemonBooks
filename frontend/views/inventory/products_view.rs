//! List products and add new ones.

use rust_decimal::Decimal;

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::products::Product;
use crate::backend::services::inventory_service::InventoryService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    products: Vec<Product>,
    sku_input: String,
    name_input: String,
    price_input: String,
    search: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SkuChanged(String),
    NameChanged(String),
    PriceChanged(String),
    Create,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
}

pub fn load(ctx: &Ctx) -> State {
    let service = InventoryService::new(ctx.db.clone());
    match service.list_products(ctx.company_id) {
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
            let service = InventoryService::new(ctx.db.clone());
            match service.create_product(&product) {
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
    let header = page_header::view("Products", "Everything you sell, in one place.", "Add product", Message::Create);

    let toolbar = toolbar::view(
        &["All Products"],
        0,
        Message::TabSelected,
        &state.search,
        "Search products",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view("Product list", "Every product on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Product> =
        state.products.iter().filter(|product| query.is_empty() || product.name.to_lowercase().contains(&query) || product.sku.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Product", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("SKU", Length::FillPortion(1)),
        ColumnDef::new("Sale Price", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|product| {
            Row::new(vec![
                Cell::text(product.name.clone()),
                Cell::text(product.sku.clone()),
                Cell::text(product.sale_price.to_string()),
                if product.is_active { Cell::status("Active", BadgeTone::Positive) } else { Cell::status("Inactive", BadgeTone::Neutral) },
            ])
        })
        .collect();

    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), |_checked| Message::RowAction(0), Message::RowAction, Message::RowAction);

    let form = column![
        text("Add a product").size(13),
        text_input("SKU", &state.sku_input).on_input(Message::SkuChanged).width(Length::Fill).padding(8),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        text_input("Sale price", &state.price_input).on_input(Message::PriceChanged).width(Length::Fill).padding(8),
        button(text("Add product")).on_press(Message::Create).padding(8),
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

        update(&mut state, Message::SkuChanged("WIDGET-1".into()), &ctx);
        update(&mut state, Message::NameChanged("Widget".into()), &ctx);
        update(&mut state, Message::PriceChanged("19.99".into()), &ctx);

        assert_eq!(state.sku_input, "WIDGET-1");
        assert_eq!(state.name_input, "Widget");
        assert_eq!(state.price_input, "19.99");
    }

    #[test]
    fn creating_with_a_non_numeric_price_sets_an_error() {
        let ctx = ctx_with_company();
        let mut state = State { sku_input: "WIDGET-1".into(), name_input: "Widget".into(), price_input: "not a number".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("sale price is not a valid number"));
        assert!(state.products.is_empty());
    }

    #[test]
    fn creating_with_a_blank_price_defaults_to_zero_and_still_succeeds() {
        let ctx = ctx_with_company();
        let mut state = State { sku_input: "WIDGET-1".into(), name_input: "Widget".into(), price_input: "".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.products.len(), 1);
        assert_eq!(state.products[0].sale_price, rust_decimal::Decimal::ZERO);
    }

    #[test]
    fn creating_a_valid_product_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State { sku_input: "WIDGET-1".into(), name_input: "Widget".into(), price_input: "19.99".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.products.len(), 1);
        assert_eq!(state.products[0].sku, "WIDGET-1");
        assert!(state.sku_input.is_empty());
        assert!(state.name_input.is_empty());
        assert!(state.price_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("widget".into()), &ctx);

        assert_eq!(state.search, "widget");
    }
}
