//! List warehouses, add new ones, edit or delete existing ones (singly
//! or in bulk), and export a selection to CSV.

use std::collections::HashSet;

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::inventory::warehouses::Warehouse;
use crate::backend::services::inventory_service::InventoryService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::toolbar::{self, StatusFilter};
use crate::frontend::components::common::{page_header, section_heading, side_panel};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

const COLUMN_LABELS: [&str; 3] = ["Warehouse", "Code", "Status"];
const DEFAULT_COLUMN_VISIBLE: [bool; 3] = [true, true, true];

#[derive(Debug, Clone, Default)]
pub struct State {
    warehouses: Vec<Warehouse>,
    name_input: String,
    code_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// warehouse instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    status_filter: StatusFilter,
    columns_open: bool,
    column_visible: Vec<bool>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    CodeChanged(String),
    Create,
    EditRow(i64),
    CancelEdit,
    RowSelected(i64, bool),
    SelectAllChanged(Vec<i64>, bool),
    ClearSelection,
    BulkDelete,
    BulkExportCsv,
    SearchChanged(String),
    StatusFilterChanged(StatusFilter),
    ResetAll,
    ToggleColumns,
    ColumnToggled(usize),
    SelectAllColumns,
    DeselectAllColumns,
    ResetColumns,
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
            state.message = None;
            state.error = None;

            if let Some(id) = state.editing_id {
                let Some(existing) = state.warehouses.iter().find(|w| w.id == Some(id)).cloned() else {
                    state.error = Some("that warehouse no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let warehouse = Warehouse { name: state.name_input.clone(), code: state.code_input.clone(), ..existing };
                if let Err(err) = warehouse.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = InventoryService::new(ctx.db.clone());
                match service.update_warehouse(&warehouse) {
                    Ok(()) => {
                        if let Some(slot) = state.warehouses.iter_mut().find(|w| w.id == Some(id)) {
                            *slot = warehouse;
                        }
                        state.editing_id = None;
                        state.name_input.clear();
                        state.code_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

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
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(warehouse) = state.warehouses.iter().find(|w| w.id == Some(id)) {
                state.editing_id = Some(id);
                state.name_input = warehouse.name.clone();
                state.code_input = warehouse.code.clone();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.name_input.clear();
            state.code_input.clear();
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
                match service.delete_warehouse(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.warehouses.retain(|w| w.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("warehouse #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} warehouse(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let query = state.search.trim().to_lowercase();
            let selected: Vec<&Warehouse> = if state.selected.is_empty() {
                state.warehouses.iter().filter(|w| matches_filters(w, &query, state.status_filter)).collect()
            } else {
                state.warehouses.iter().filter(|w| state.selected.contains(&w.id.unwrap_or_default())).collect()
            };
            let headers = ["Name", "Code", "Status"];
            let rows: Vec<Vec<String>> =
                selected.iter().map(|w| vec![w.name.clone(), w.code.clone(), if w.is_active { "Active".to_string() } else { "Inactive".to_string() }]).collect();
            let path = ctx.export_path("warehouses_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} warehouse(s) to {}", selected.len(), path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => state.search = value,
        Message::StatusFilterChanged(filter) => state.status_filter = filter,
        Message::ResetAll => {
            state.search.clear();
            state.status_filter = StatusFilter::All;
        }
        Message::ToggleColumns => state.columns_open = !state.columns_open,
        Message::ColumnToggled(index) => {
            if index >= state.column_visible.len() {
                state.column_visible.resize(index + 1, true);
            }
            state.column_visible[index] = !state.column_visible[index];
        }
        Message::SelectAllColumns => state.column_visible = vec![true; COLUMN_LABELS.len()],
        Message::DeselectAllColumns => state.column_visible = vec![false; COLUMN_LABELS.len()],
        Message::ResetColumns => state.column_visible = DEFAULT_COLUMN_VISIBLE.to_vec(),
    }
}

/// Whether a warehouse passes both the search box and the status
/// filter -- shared by the table's live filtering in `view` and by
/// `BulkExportCsv`'s "no selection -> export everything currently
/// visible" fallback, so the two can never disagree.
fn matches_filters(warehouse: &Warehouse, query: &str, status: StatusFilter) -> bool {
    status.matches(warehouse.is_active) && (query.is_empty() || warehouse.name.to_lowercase().contains(query) || warehouse.code.to_lowercase().contains(query))
}

const TITLE: &str = "Warehouses";
const DESCRIPTION: &str = "Every stock location on this company's books.";

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add warehouse" };
    let header = container(page_header::view(theme, TITLE, DESCRIPTION, header_label, Message::Create)).padding(iced::Padding { top: 24.0, right: 24.0, bottom: 0.0, left: 24.0 });

    column![header, body(state, theme)].spacing(20).into()
}

/// Same as [`view`], except the title sits inline with `switcher` (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) on the
/// title's own row instead of alone -- `wrap` converts this module's
/// own `Message` into whatever the caller's `switcher` produces, so the
/// two can share one `Element` tree despite having different message
/// types.
pub fn view_with_switcher<'a, ParentMessage: Clone + 'a>(state: &'a State, theme: Theme, switcher: Element<'a, ParentMessage>, wrap: impl Fn(Message) -> ParentMessage + 'a) -> Element<'a, ParentMessage> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add warehouse" };
    let header =
        container(page_header::view_with_switcher(theme, TITLE, DESCRIPTION, switcher, header_label, wrap(Message::Create))).padding(iced::Padding { top: 24.0, right: 24.0, bottom: 0.0, left: 24.0 });

    column![header, body(state, theme).map(wrap)].spacing(20).into()
}

fn body(state: &State, theme: Theme) -> Element<'_, Message> {
    let toolbar = toolbar::list_toolbar(
        theme,
        &state.search,
        "Search warehouses",
        Message::SearchChanged,
        state.status_filter,
        Message::StatusFilterChanged,
        Message::ResetAll,
        Message::ToggleColumns,
        Message::BulkExportCsv,
        !state.selected.is_empty(),
        Message::BulkDelete,
    );

    let heading = section_heading::view(theme, "Warehouse list", "Every warehouse on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Warehouse> = state.warehouses.iter().filter(|warehouse| matches_filters(warehouse, &query, state.status_filter)).collect();

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

    let row_ids: Vec<i64> = filtered.iter().map(|w| w.id.unwrap_or_default()).collect();
    let select_ids = row_ids.clone();
    let selected_flags: Vec<bool> = row_ids.iter().map(|id| state.selected.contains(id)).collect();
    let edit_ids = row_ids.clone();
    let edit_ids_primary = row_ids.clone();

    let table = data_table::view_with_column_visibility(
        theme,
        columns,
        &state.column_visible,
        rows,
        Some(selected_flags),
        move |index, checked| Message::RowSelected(row_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::EditRow(edit_ids[index]),
        move |index| Message::EditRow(edit_ids_primary[index]),
    );

    let form_heading = if state.editing_id.is_some() { "Edit warehouse" } else { "Add a warehouse" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("Code", &state.code_input).style(inputs::text(theme)).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
        text_input("Name", &state.name_input).style(inputs::text(theme)).on_input(Message::NameChanged).width(Length::Fill).padding(8),
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
        form = form.push(button(text("Add warehouse")).style(button_style::primary(theme)).on_press(Message::Create).padding(8));
    }

    let mut content = column![toolbar].spacing(20).padding(iced::Padding { top: 0.0, right: 24.0, bottom: 24.0, left: 24.0 }).width(Length::Fill);

    content = content.push(heading).push(table).push(form);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    let base: Element<'_, Message> = container(scrollable(content).height(Length::Fill)).width(Length::Fill).into();

    let panel = if state.columns_open {
        Some(data_table::columns_panel(theme, &COLUMN_LABELS, &state.column_visible, Message::ColumnToggled, Message::SelectAllColumns, Message::DeselectAllColumns, Message::ResetColumns))
    } else {
        None
    };

    side_panel::split(theme, base, panel)
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
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_warehouses_view_test_{:?}", std::thread::current().id())), ..ctx }
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

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_warehouse() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Main Warehouse".into(), code_input: "MAIN".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.warehouses[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.name_input, "Main Warehouse");
        assert_eq!(state.code_input, "MAIN");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_warehouse_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Main Warehouse".into(), code_input: "MAIN".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.warehouses[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Central Warehouse".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.warehouses.len(), 1);
        assert_eq!(state.warehouses[0].name, "Central Warehouse");

        let service = InventoryService::new(ctx.db.clone());
        let reloaded = service.list_warehouses(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].name, "Central Warehouse");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_warehouse() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Main Warehouse".into(), code_input: "MAIN".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.warehouses[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.warehouses[0].name, "Main Warehouse");
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
    fn bulk_delete_removes_every_selected_unreferenced_warehouse() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Main Warehouse".into()), &ctx);
        state.code_input = "MAIN".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Overflow Warehouse".into();
        state.code_input = "OVERFLOW".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.warehouses.iter().map(|w| w.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.warehouses.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes a warehouse still referenced by a stock
    /// movement: the unreferenced one is deleted, the referenced one is
    /// skipped with a clear per-record error in the summary -- bulk
    /// delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_warehouse_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::inventory::products::Product;
        use crate::backend::inventory::stock_movements::{MovementType, StockMovement};
        use crate::backend::repositories::inventory_repository::InventoryRepository;

        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Main Warehouse".into()), &ctx);
        state.code_input = "MAIN".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Overflow Warehouse".into();
        state.code_input = "OVERFLOW".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.warehouses.iter().find(|w| w.code == "MAIN").unwrap().id.unwrap();
        let free_id = state.warehouses.iter().find(|w| w.code == "OVERFLOW").unwrap().id.unwrap();

        ctx.db.run_migrations(crate::backend::repositories::accounting_repository::MIGRATIONS).unwrap();
        let inventory = InventoryRepository::new(ctx.db.clone());
        let product = inventory.create_product(&Product::new(ctx.company_id, "WIDGET-1", "Widget")).unwrap();
        inventory
            .record_movement(&StockMovement::new(ctx.company_id, product.id.unwrap(), referenced_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), MovementType::Purchase, rust_decimal_macros::dec!(100)))
            .unwrap();

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.warehouses.len(), 1);
        assert_eq!(state.warehouses[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_warehouses() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Main Warehouse".into()), &ctx);
        state.code_input = "MAIN".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Overflow Warehouse".into();
        state.code_input = "OVERFLOW".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.warehouses.iter().find(|w| w.code == "MAIN").unwrap().id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("warehouses_export.csv")).unwrap();
        assert!(contents.contains("Main Warehouse") && !contents.contains("Overflow Warehouse"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_selection_exports_every_currently_filtered_warehouse() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Main Warehouse".into()), &ctx);
        state.code_input = "MAIN".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Main Annex".into();
        state.code_input = "ANNEX".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Overflow Warehouse".into();
        state.code_input = "OVERFLOW".into();
        update(&mut state, Message::Create, &ctx);
        state.search = "main".into();

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("warehouses_export.csv")).unwrap();
        assert!(contents.contains("Main Warehouse") && contents.contains("Main Annex"));
        assert!(!contents.contains("Overflow Warehouse"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_and_status_filter() {
        let ctx = ctx_with_company();
        let mut state = State { search: "main".into(), status_filter: StatusFilter::Inactive, ..Default::default() };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.status_filter, StatusFilter::All);
    }

    #[test]
    fn status_filter_narrows_to_only_matching_warehouses() {
        let mut active = Warehouse::new(1, "Main Warehouse", "MAIN");
        active.is_active = true;
        let mut inactive = Warehouse::new(1, "Old Warehouse", "OLD");
        inactive.is_active = false;

        assert!(matches_filters(&active, "", StatusFilter::All));
        assert!(matches_filters(&active, "", StatusFilter::Active));
        assert!(!matches_filters(&active, "", StatusFilter::Inactive));

        assert!(matches_filters(&inactive, "", StatusFilter::All));
        assert!(!matches_filters(&inactive, "", StatusFilter::Active));
        assert!(matches_filters(&inactive, "", StatusFilter::Inactive));
    }

    #[test]
    fn column_toggled_flips_that_columns_visibility() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::ColumnToggled(2), &ctx);
        assert_eq!(state.column_visible, vec![true, true, false]);

        update(&mut state, Message::ColumnToggled(2), &ctx);
        assert_eq!(state.column_visible, vec![true, true, true]);
    }

    #[test]
    fn select_all_deselect_all_and_reset_columns_set_the_expected_visibility() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::DeselectAllColumns, &ctx);
        assert_eq!(state.column_visible, vec![false, false, false]);

        update(&mut state, Message::ColumnToggled(0), &ctx);
        update(&mut state, Message::ResetColumns, &ctx);
        assert_eq!(state.column_visible, DEFAULT_COLUMN_VISIBLE.to_vec());

        update(&mut state, Message::DeselectAllColumns, &ctx);
        update(&mut state, Message::SelectAllColumns, &ctx);
        assert_eq!(state.column_visible, vec![true, true, true]);
    }
}
