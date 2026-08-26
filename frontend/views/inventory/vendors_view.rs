//! List vendors, add new ones, edit or delete existing ones (singly or
//! in bulk), and export a selection to CSV. Grouped under `inventory/`
//! rather than a dedicated `purchases/` directory -- the frozen
//! frontend/views/ structure (Section 2) only has nine subdirectories
//! and none of them is `purchases/`; purchasing pairs naturally with
//! inventory (goods bought feed stock) in the same way it does in most
//! ERP navigation.

use std::collections::HashSet;

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::parties::vendors::Vendor;
use crate::backend::services::party_service::PartyService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::toolbar::{self, StatusFilter};
use crate::frontend::components::common::{page_header, section_heading, side_panel};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::views::Ctx;

const COLUMN_LABELS: [&str; 2] = ["Vendor", "Status"];

#[derive(Debug, Clone, Default)]
pub struct State {
    vendors: Vec<Vendor>,
    name_input: String,
    email_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// vendor instead of creating a new one -- `Message::Create` does
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
    EmailChanged(String),
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
            state.message = None;
            state.error = None;

            if let Some(id) = state.editing_id {
                let Some(existing) = state.vendors.iter().find(|v| v.id == Some(id)).cloned() else {
                    state.error = Some("that vendor no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let vendor = Vendor {
                    name: state.name_input.clone(),
                    email: if state.email_input.trim().is_empty() { None } else { Some(state.email_input.clone()) },
                    ..existing
                };
                if let Err(err) = vendor.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = PartyService::new(ctx.db.clone());
                match service.update_vendor(&vendor) {
                    Ok(()) => {
                        if let Some(slot) = state.vendors.iter_mut().find(|v| v.id == Some(id)) {
                            *slot = vendor;
                        }
                        state.editing_id = None;
                        state.name_input.clear();
                        state.email_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

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
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(vendor) = state.vendors.iter().find(|v| v.id == Some(id)) {
                state.editing_id = Some(id);
                state.name_input = vendor.name.clone();
                state.email_input = vendor.email.clone().unwrap_or_default();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.name_input.clear();
            state.email_input.clear();
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
            let service = PartyService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_vendor(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.vendors.retain(|v| v.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("vendor #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} vendor(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let query = state.search.trim().to_lowercase();
            let selected: Vec<&Vendor> = if state.selected.is_empty() {
                state.vendors.iter().filter(|v| matches_filters(v, &query, state.status_filter)).collect()
            } else {
                state.vendors.iter().filter(|v| state.selected.contains(&v.id.unwrap_or_default())).collect()
            };
            let headers = ["Name", "Email", "Phone", "Payment Terms (days)", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|v| {
                    vec![
                        v.name.clone(),
                        v.email.clone().unwrap_or_default(),
                        v.phone.clone().unwrap_or_default(),
                        v.payment_terms_days.to_string(),
                        if v.is_active { "Active".to_string() } else { "Inactive".to_string() },
                    ]
                })
                .collect();
            let path = ctx.export_path("vendors_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} vendor(s) to {}", selected.len(), path.display())),
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
    }
}

/// Whether a vendor passes both the search box and the status filter --
/// shared by the table's live filtering in `view` and by
/// `BulkExportCsv`'s "no selection -> export everything currently
/// visible" fallback, so the two can never disagree.
fn matches_filters(vendor: &Vendor, query: &str, status: StatusFilter) -> bool {
    status.matches(vendor.is_active) && (query.is_empty() || vendor.name.to_lowercase().contains(query) || vendor.email.as_deref().unwrap_or_default().to_lowercase().contains(query))
}

const TITLE: &str = "Vendors";
const DESCRIPTION: &str = "Everyone you buy from, in one place.";

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add vendor" };
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
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add vendor" };
    let header =
        container(page_header::view_with_switcher(theme, TITLE, DESCRIPTION, switcher, header_label, wrap(Message::Create))).padding(iced::Padding { top: 24.0, right: 24.0, bottom: 0.0, left: 24.0 });

    column![header, body(state, theme).map(wrap)].spacing(20).into()
}

fn body(state: &State, theme: Theme) -> Element<'_, Message> {
    let toolbar = toolbar::list_toolbar(
        theme,
        &state.search,
        "Search vendors",
        Message::SearchChanged,
        state.status_filter,
        Message::StatusFilterChanged,
        Message::ResetAll,
        Message::ToggleColumns,
        Message::BulkExportCsv,
        !state.selected.is_empty(),
        Message::BulkDelete,
    );

    let heading = section_heading::view(theme, "Vendor list", "Every vendor on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Vendor> = state.vendors.iter().filter(|vendor| matches_filters(vendor, &query, state.status_filter)).collect();

    let columns = vec![
        ColumnDef::new("Vendor", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Status", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|vendor| {
            Row::new(vec![
                Cell::person(vendor.name.clone(), vendor.email.clone().unwrap_or_default()),
                if vendor.is_active { Cell::status("Active", BadgeTone::Positive) } else { Cell::status("Inactive", BadgeTone::Neutral) },
            ])
        })
        .collect();

    let row_ids: Vec<i64> = filtered.iter().map(|v| v.id.unwrap_or_default()).collect();
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

    let form_heading = if state.editing_id.is_some() { "Edit vendor" } else { "Add a vendor" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("Vendor name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        text_input("Email (optional)", &state.email_input).on_input(Message::EmailChanged).width(Length::Fill).padding(8),
    ]
    .spacing(8);

    if state.editing_id.is_some() {
        form = form.push(
            iced::widget::row![
                button(text("Save changes")).on_press(Message::Create).padding(8),
                button(text("Cancel")).on_press(Message::CancelEdit).padding(8),
            ]
            .spacing(8),
        );
    } else {
        form = form.push(button(text("Add vendor")).on_press(Message::Create).padding(8));
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

    if state.columns_open {
        side_panel::overlay(theme, base, data_table::columns_panel(theme, &COLUMN_LABELS, &state.column_visible, Message::ColumnToggled), Message::ToggleColumns)
    } else {
        base
    }
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

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_vendors_view_test_{:?}", std::thread::current().id())), ..ctx }
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

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("acme".into()), &ctx);

        assert_eq!(state.search, "acme");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_vendor() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Supplies".into(), email_input: "ap@acmesupplies.test".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.vendors[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.name_input, "Acme Supplies");
        assert_eq!(state.email_input, "ap@acmesupplies.test");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_vendor_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Supplies".into(), email_input: "".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.vendors[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Acme Supplies Renamed".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.vendors.len(), 1);
        assert_eq!(state.vendors[0].name, "Acme Supplies Renamed");

        let service = PartyService::new(ctx.db.clone());
        let reloaded = service.list_vendors(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].name, "Acme Supplies Renamed");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_vendor() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Supplies".into(), email_input: "".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.vendors[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.vendors[0].name, "Acme Supplies");
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
    fn bulk_delete_removes_every_selected_unreferenced_vendor() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Supplies".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Manufacturing".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.vendors.iter().map(|v| v.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.vendors.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes a vendor still referenced by a bill:
    /// the unreferenced one is deleted, the referenced one is skipped
    /// with a clear per-record error in the summary -- bulk delete does
    /// not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_vendor_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::purchases::bills::{Bill, BillLine};
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::purchase_repository::{self, PurchaseRepository};

        let ctx = ctx_with_company();
        ctx.db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        ctx.db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        ctx.db.run_migrations(purchase_repository::MIGRATIONS).unwrap();

        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Supplies".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Manufacturing".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.vendors[0].id.unwrap();
        let free_id = state.vendors[1].id.unwrap();

        let accounting = AccountingRepository::new(ctx.db.clone());
        accounting
            .create_fiscal_period(&FiscalPeriod::new(ctx.company_id, "Open Period", chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap()))
            .unwrap();
        let ap = accounting.create_account(&Account::new(ctx.company_id, "2000", "Accounts Payable", AccountType::Liability)).unwrap();
        let expense = accounting.create_account(&Account::new(ctx.company_id, "6000", "Office Supplies", AccountType::Expense)).unwrap();
        let purchases = PurchaseRepository::new(ctx.db.clone());
        let mut bill = Bill::new(ctx.company_id, referenced_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(), chrono::NaiveDate::from_ymd_opt(2026, 2, 4).unwrap());
        bill.lines.push(BillLine::new(expense.id.unwrap(), "Office supplies", rust_decimal_macros::dec!(1), rust_decimal_macros::dec!(300)));
        let created = purchases.create_bill(&bill).unwrap();
        purchases.post_bill(created.id.unwrap(), ap.id.unwrap()).unwrap();

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.vendors.len(), 1);
        assert_eq!(state.vendors[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_vendors() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Supplies".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Manufacturing".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.vendors[0].id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("vendors_export.csv")).unwrap();
        assert!(contents.contains("Acme Supplies"));
        assert!(!contents.contains("Beta Manufacturing"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_selection_exports_every_currently_filtered_vendor() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Supplies".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Acme Distribution".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Manufacturing".into();
        update(&mut state, Message::Create, &ctx);
        state.search = "acme".into();

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("vendors_export.csv")).unwrap();
        assert!(contents.contains("Acme Supplies") && contents.contains("Acme Distribution"));
        assert!(!contents.contains("Beta Manufacturing"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_and_status_filter() {
        let ctx = ctx_with_company();
        let mut state = State { search: "acme".into(), status_filter: StatusFilter::Inactive, ..Default::default() };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.status_filter, StatusFilter::All);
    }

    #[test]
    fn status_filter_narrows_to_only_matching_vendors() {
        let mut active = Vendor::new(1, "Acme Supplies");
        active.is_active = true;
        let mut inactive = Vendor::new(1, "Old Supplier");
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

        update(&mut state, Message::ColumnToggled(1), &ctx);
        assert_eq!(state.column_visible, vec![true, false]);

        update(&mut state, Message::ColumnToggled(1), &ctx);
        assert_eq!(state.column_visible, vec![true, true]);
    }
}
