//! List products, add new ones, edit or delete existing ones (singly or
//! in bulk), and export a selection to CSV.

use std::collections::HashSet;

use rust_decimal::Decimal;

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::products::Product;
use crate::backend::services::inventory_service::InventoryService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    products: Vec<Product>,
    sku_input: String,
    name_input: String,
    price_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// product instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SkuChanged(String),
    NameChanged(String),
    PriceChanged(String),
    Create,
    EditRow(i64),
    CancelEdit,
    RowSelected(i64, bool),
    SelectAllChanged(Vec<i64>, bool),
    ClearSelection,
    BulkDelete,
    BulkExportCsv,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
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
            state.message = None;
            state.error = None;
            let price: Decimal = if state.price_input.trim().is_empty() {
                Decimal::ZERO
            } else {
                match state.price_input.parse() {
                    Ok(price) => price,
                    Err(_) => {
                        state.error = Some("sale price is not a valid number".into());
                        return;
                    }
                }
            };

            if let Some(id) = state.editing_id {
                let Some(existing) = state.products.iter().find(|p| p.id == Some(id)).cloned() else {
                    state.error = Some("that product no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let product = Product { sku: state.sku_input.clone(), name: state.name_input.clone(), sale_price: price, ..existing };
                if let Err(err) = product.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = InventoryService::new(ctx.db.clone());
                match service.update_product(&product) {
                    Ok(()) => {
                        if let Some(slot) = state.products.iter_mut().find(|p| p.id == Some(id)) {
                            *slot = product;
                        }
                        state.editing_id = None;
                        state.sku_input.clear();
                        state.name_input.clear();
                        state.price_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut product = Product::new(ctx.company_id, state.sku_input.clone(), state.name_input.clone());
            product.sale_price = price;
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
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(product) = state.products.iter().find(|p| p.id == Some(id)) {
                state.editing_id = Some(id);
                state.sku_input = product.sku.clone();
                state.name_input = product.name.clone();
                state.price_input = product.sale_price.to_string();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.sku_input.clear();
            state.name_input.clear();
            state.price_input.clear();
            state.error = None;
        }
        Message::RowSelected(id, checked) => {
            if checked {
                state.selected.insert(id);
            } else {
                state.selected.remove(&id);
            }
        }
        Message::SelectAllChanged(ids, checked) => {
            for id in ids {
                if checked {
                    state.selected.insert(id);
                } else {
                    state.selected.remove(&id);
                }
            }
        }
        Message::ClearSelection => state.selected.clear(),
        Message::BulkDelete => {
            state.message = None;
            state.error = None;
            let service = InventoryService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_product(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.products.retain(|p| p.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("product #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} product(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&Product> = state.products.iter().filter(|p| state.selected.contains(&p.id.unwrap_or_default())).collect();
            let headers = ["SKU", "Name", "Sale Price", "Status"];
            let rows: Vec<Vec<String>> =
                selected.iter().map(|p| vec![p.sku.clone(), p.name.clone(), p.sale_price.to_string(), if p.is_active { "Active".to_string() } else { "Inactive".to_string() }]).collect();
            let path = ctx.export_path("products_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} product(s) to {}", selected.len(), path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, and "Filters" / "Sort by" are
        // rendered by the shared toolbar component now (Section 6) --
        // wiring real tab/filter/sort behavior is follow-up work, not
        // part of this pass.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked => {}
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add product" };
    let header = page_header::view(theme, "Products", "Everything you sell, in one place.", header_label, Message::Create);

    let toolbar = toolbar::view(
        theme,
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

    let heading = section_heading::view(theme, "Product list", "Every product on this company's books.");

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

    let row_ids: Vec<i64> = filtered.iter().map(|p| p.id.unwrap_or_default()).collect();
    let select_ids = row_ids.clone();
    let selected_flags: Vec<bool> = row_ids.iter().map(|id| state.selected.contains(id)).collect();
    let edit_ids = row_ids.clone();
    let edit_ids_primary = row_ids.clone();

    let table = data_table::view(
        theme,
        columns,
        rows,
        Some(selected_flags),
        move |index, checked| Message::RowSelected(row_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::EditRow(edit_ids[index]),
        move |index| Message::EditRow(edit_ids_primary[index]),
    );

    let form_heading = if state.editing_id.is_some() { "Edit product" } else { "Add a product" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("SKU", &state.sku_input).on_input(Message::SkuChanged).width(Length::Fill).padding(8),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        text_input("Sale price", &state.price_input).on_input(Message::PriceChanged).width(Length::Fill).padding(8),
    ]
    .spacing(8);

    if state.editing_id.is_some() {
        form = form.push(
            iced::widget::row![
                button(text("Save changes")).style(button_style::primary(theme)).on_press(Message::Create).padding(8),
                button(text("Cancel")).style(button_style::secondary(theme)).on_press(Message::CancelEdit).padding(8),
            ]
            .spacing(8),
        );
    } else {
        form = form.push(button(text("Add product")).style(button_style::primary(theme)).on_press(Message::Create).padding(8));
    }

    let mut content = column![header, toolbar, heading].spacing(20).padding(24).width(Length::Fill);

    if !state.selected.is_empty() {
        content = content.push(bulk_action_bar::view(theme, state.selected.len(), Message::BulkDelete, Message::BulkExportCsv, Message::ClearSelection));
    }

    content = content.push(table).push(form);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
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

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_products_view_test_{:?}", std::thread::current().id())), ..ctx }
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

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_product() {
        let ctx = ctx_with_company();
        let mut state = State { sku_input: "WIDGET-1".into(), name_input: "Widget".into(), price_input: "19.99".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.products[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.sku_input, "WIDGET-1");
        assert_eq!(state.name_input, "Widget");
        assert_eq!(state.price_input, "19.99");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_product_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State { sku_input: "WIDGET-1".into(), name_input: "Widget".into(), price_input: "19.99".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.products[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Deluxe Widget".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.products.len(), 1);
        assert_eq!(state.products[0].name, "Deluxe Widget");

        let service = InventoryService::new(ctx.db.clone());
        let reloaded = service.list_products(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].name, "Deluxe Widget");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_product() {
        let ctx = ctx_with_company();
        let mut state = State { sku_input: "WIDGET-1".into(), name_input: "Widget".into(), price_input: "19.99".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.products[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.products[0].name, "Widget");
    }

    #[test]
    fn row_selected_toggles_membership_in_the_selection() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::RowSelected(1, true), &ctx);
        update(&mut state, Message::RowSelected(2, true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2]));

        update(&mut state, Message::RowSelected(1, false), &ctx);
        assert_eq!(state.selected, HashSet::from([2]));
    }

    #[test]
    fn select_all_changed_selects_or_clears_every_given_id() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SelectAllChanged(vec![1, 2, 3], true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2, 3]));

        update(&mut state, Message::SelectAllChanged(vec![1, 2, 3], false), &ctx);
        assert!(state.selected.is_empty());
    }

    #[test]
    fn clear_selection_empties_it_regardless_of_what_was_selected() {
        let ctx = ctx_with_company();
        let mut state = State { selected: HashSet::from([1, 2, 3]), ..Default::default() };

        update(&mut state, Message::ClearSelection, &ctx);

        assert!(state.selected.is_empty());
    }

    #[test]
    fn bulk_delete_removes_every_selected_unreferenced_product() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::SkuChanged("WIDGET-1".into()), &ctx);
        state.name_input = "Widget".into();
        update(&mut state, Message::Create, &ctx);
        state.sku_input = "GADGET-1".into();
        state.name_input = "Gadget".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.products.iter().map(|p| p.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.products.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes a product still referenced by a stock
    /// movement: the unreferenced one is deleted, the referenced one is
    /// skipped with a clear per-record error in the summary -- bulk
    /// delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_product_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::inventory::stock_movements::{MovementType, StockMovement};
        use crate::backend::inventory::warehouses::Warehouse;
        use crate::backend::repositories::inventory_repository::InventoryRepository;

        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::SkuChanged("WIDGET-1".into()), &ctx);
        state.name_input = "Widget".into();
        update(&mut state, Message::Create, &ctx);
        state.sku_input = "GADGET-1".into();
        state.name_input = "Gadget".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.products.iter().find(|p| p.sku == "WIDGET-1").unwrap().id.unwrap();
        let free_id = state.products.iter().find(|p| p.sku == "GADGET-1").unwrap().id.unwrap();

        ctx.db.run_migrations(crate::backend::repositories::accounting_repository::MIGRATIONS).unwrap();
        let inventory = InventoryRepository::new(ctx.db.clone());
        let warehouse = inventory.create_warehouse(&Warehouse::new(ctx.company_id, "Main Warehouse", "MAIN")).unwrap();
        inventory
            .record_movement(&StockMovement::new(ctx.company_id, referenced_id, warehouse.id.unwrap(), chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), MovementType::Purchase, rust_decimal_macros::dec!(100)))
            .unwrap();

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.products.len(), 1);
        assert_eq!(state.products[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_products() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::SkuChanged("WIDGET-1".into()), &ctx);
        state.name_input = "Widget".into();
        update(&mut state, Message::Create, &ctx);
        state.sku_input = "GADGET-1".into();
        state.name_input = "Gadget".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.products.iter().find(|p| p.sku == "WIDGET-1").unwrap().id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("products_export.csv")).unwrap();
        assert!(contents.contains("Widget") && !contents.contains("Gadget"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
