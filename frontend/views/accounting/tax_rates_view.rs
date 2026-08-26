//! List tax rates, add new ones, edit or delete existing ones (singly
//! or in bulk), and export a selection to CSV. Grouped under
//! `accounting/` -- the frozen frontend/views/ structure (Section 2) has
//! no dedicated `tax/` directory, and tax rates are closely tied to the
//! chart of accounts.

use std::collections::HashSet;

use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::validation::Validate;
use crate::backend::services::tax_service::TaxService;
use crate::backend::tax::tax_rates::{TaxDirection, TaxRate};
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    id: i64,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

const DIRECTIONS: [TaxDirection; 2] = [TaxDirection::Output, TaxDirection::Input];

#[derive(Debug, Clone, Default)]
pub struct State {
    tax_rates: Vec<TaxRate>,
    gl_accounts: Vec<Choice>,
    gl_account: Option<Choice>,
    direction: Option<TaxDirection>,
    name_input: String,
    rate_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// tax rate instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    RateChanged(String),
    GlAccountSelected(Choice),
    DirectionSelected(TaxDirection),
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
    let service = TaxService::new(ctx.db.clone());

    let tax_rates = service.list_tax_rates(ctx.company_id).unwrap_or_default();
    let gl_accounts = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    State { tax_rates, gl_accounts, ..Default::default() }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::RateChanged(value) => state.rate_input = value,
        Message::GlAccountSelected(choice) => state.gl_account = Some(choice),
        Message::DirectionSelected(direction) => state.direction = Some(direction),
        Message::Create => {
            state.message = None;
            state.error = None;
            let (Some(gl_account), Some(direction)) = (&state.gl_account, state.direction) else {
                state.error = Some("choose a GL account and a direction".into());
                return;
            };
            let rate_percent: Decimal = match state.rate_input.parse() {
                Ok(rate) => rate,
                Err(_) => {
                    state.error = Some("rate is not a valid number".into());
                    return;
                }
            };

            if let Some(id) = state.editing_id {
                let Some(existing) = state.tax_rates.iter().find(|r| r.id == Some(id)).cloned() else {
                    state.error = Some("that tax rate no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let tax_rate = TaxRate { name: state.name_input.clone(), rate_percent, direction, gl_account_id: gl_account.id, ..existing };
                if let Err(err) = tax_rate.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = TaxService::new(ctx.db.clone());
                match service.update_tax_rate(&tax_rate) {
                    Ok(()) => {
                        if let Some(slot) = state.tax_rates.iter_mut().find(|r| r.id == Some(id)) {
                            *slot = tax_rate;
                        }
                        state.editing_id = None;
                        state.name_input.clear();
                        state.rate_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let tax_rate = TaxRate::new(ctx.company_id, state.name_input.clone(), rate_percent, direction, gl_account.id);
            let service = TaxService::new(ctx.db.clone());
            match service.create_tax_rate(&tax_rate) {
                Ok(created) => {
                    state.tax_rates.push(created);
                    state.name_input.clear();
                    state.rate_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(rate) = state.tax_rates.iter().find(|r| r.id == Some(id)) {
                state.editing_id = Some(id);
                state.name_input = rate.name.clone();
                state.rate_input = rate.rate_percent.to_string();
                state.direction = Some(rate.direction);
                state.gl_account = state.gl_accounts.iter().find(|c| c.id == rate.gl_account_id).cloned();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.name_input.clear();
            state.rate_input.clear();
            state.direction = None;
            state.gl_account = None;
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
            let service = TaxService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_tax_rate(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.tax_rates.retain(|r| r.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("tax rate #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} tax rate(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&TaxRate> = state.tax_rates.iter().filter(|r| state.selected.contains(&r.id.unwrap_or_default())).collect();
            let headers = ["Name", "Rate %", "Direction", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|r| vec![r.name.clone(), r.rate_percent.to_string(), r.direction.as_str().to_string(), if r.is_active { "Active".to_string() } else { "Inactive".to_string() }])
                .collect();
            let path = ctx.export_path("tax_rates_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} tax rate(s) to {}", selected.len(), path.display())),
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
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add tax rate" };
    let header = page_header::view(theme, "Tax Rates", "Every tax rate on this company's books.", header_label, Message::Create);

    let toolbar = toolbar::view(
        theme,
        &["All Tax Rates"],
        0,
        Message::TabSelected,
        &state.search,
        "Search tax rates",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Tax rate list", "Every output and input tax rate configured.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&TaxRate> = state.tax_rates.iter().filter(|rate| query.is_empty() || rate.name.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Name", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Rate %", Length::FillPortion(1)),
        ColumnDef::new("Direction", Length::FillPortion(1)),
    ];

    let rows = filtered.iter().map(|rate| Row::new(vec![Cell::text(rate.name.clone()), Cell::text(rate.rate_percent.to_string()), Cell::status(rate.direction.as_str(), BadgeTone::Neutral)])).collect();

    let row_ids: Vec<i64> = filtered.iter().map(|r| r.id.unwrap_or_default()).collect();
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

    let form_heading = if state.editing_id.is_some() { "Edit tax rate" } else { "Add a tax rate" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("Name", &state.name_input).style(inputs::text(theme)).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        text_input("Rate %", &state.rate_input).style(inputs::text(theme)).on_input(Message::RateChanged).width(Length::Fill).padding(8),
        pick_list(DIRECTIONS, state.direction, Message::DirectionSelected).placeholder("Direction"),
        pick_list(state.gl_accounts.as_slice(), state.gl_account.clone(), Message::GlAccountSelected).placeholder("GL account"),
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
        form = form.push(button(text("Add tax rate")).style(button_style::primary(theme)).on_press(Message::Create).padding(8));
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
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::tax_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_a_gl_account() -> (Ctx, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(tax_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        let tax_payable = accounting_repo.create_account(&Account::new(company_id, "2200", "Tax Payable", AccountType::Liability)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        let choice = Choice { id: tax_payable.id.unwrap(), label: "2200 - Tax Payable".into() };
        (ctx, choice)
    }

    fn ctx_with_temp_export_dir() -> (Ctx, Choice) {
        let (ctx, choice) = ctx_with_a_gl_account();
        (Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_tax_rates_view_test_{:?}", std::thread::current().id())), ..ctx }, choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Sales Tax".into()), &ctx);
        update(&mut state, Message::RateChanged("15".into()), &ctx);
        update(&mut state, Message::GlAccountSelected(gl_account.clone()), &ctx);
        update(&mut state, Message::DirectionSelected(TaxDirection::Output), &ctx);

        assert_eq!(state.name_input, "Sales Tax");
        assert_eq!(state.rate_input, "15");
        assert_eq!(state.gl_account, Some(gl_account));
        assert_eq!(state.direction, Some(TaxDirection::Output));
    }

    #[test]
    fn creating_without_a_gl_account_or_direction_sets_an_error() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State { name_input: "Sales Tax".into(), rate_input: "15".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a GL account and a direction"));
    }

    #[test]
    fn creating_with_a_non_numeric_rate_sets_an_error() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            name_input: "Sales Tax".into(),
            rate_input: "not a number".into(),
            gl_account: Some(gl_account),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("rate is not a valid number"));
    }

    #[test]
    fn creating_a_valid_tax_rate_adds_it_to_the_list_and_clears_the_form() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            name_input: "Sales Tax".into(),
            rate_input: "15".into(),
            gl_account: Some(gl_account),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.tax_rates.len(), 1);
        assert_eq!(state.tax_rates[0].name, "Sales Tax");
        assert!(state.name_input.is_empty());
        assert!(state.rate_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("sales".into()), &ctx);

        assert_eq!(state.search, "sales");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_tax_rate() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            gl_accounts: vec![gl_account.clone()],
            name_input: "Sales Tax".into(),
            rate_input: "15".into(),
            gl_account: Some(gl_account.clone()),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };
        update(&mut state, Message::Create, &ctx);
        let id = state.tax_rates[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.name_input, "Sales Tax");
        assert_eq!(state.rate_input, "15");
        assert_eq!(state.direction, Some(TaxDirection::Output));
        assert_eq!(state.gl_account, Some(gl_account));
    }

    #[test]
    fn saving_an_edit_updates_the_existing_tax_rate_in_place_and_clears_editing_state() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            gl_accounts: vec![gl_account.clone()],
            name_input: "Sales Tax".into(),
            rate_input: "15".into(),
            gl_account: Some(gl_account),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };
        update(&mut state, Message::Create, &ctx);
        let id = state.tax_rates[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.rate_input = "16".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.tax_rates.len(), 1);
        assert_eq!(state.tax_rates[0].rate_percent, rust_decimal_macros::dec!(16));

        let service = TaxService::new(ctx.db.clone());
        let reloaded = service.list_tax_rates(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].rate_percent, rust_decimal_macros::dec!(16));
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_tax_rate() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            gl_accounts: vec![gl_account.clone()],
            name_input: "Sales Tax".into(),
            rate_input: "15".into(),
            gl_account: Some(gl_account),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };
        update(&mut state, Message::Create, &ctx);
        let id = state.tax_rates[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.rate_input = "99".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert_eq!(state.tax_rates[0].rate_percent, rust_decimal_macros::dec!(15));
    }

    #[test]
    fn row_selected_toggles_membership_in_the_selection() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::RowSelected(1, true), &ctx);
        update(&mut state, Message::RowSelected(2, true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2]));

        update(&mut state, Message::RowSelected(1, false), &ctx);
        assert_eq!(state.selected, HashSet::from([2]));
    }

    #[test]
    fn select_all_changed_selects_or_clears_every_given_id() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::SelectAllChanged(vec![1, 2, 3], true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2, 3]));

        update(&mut state, Message::SelectAllChanged(vec![1, 2, 3], false), &ctx);
        assert!(state.selected.is_empty());
    }

    #[test]
    fn clear_selection_empties_it_regardless_of_what_was_selected() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State { selected: HashSet::from([1, 2, 3]), ..Default::default() };

        update(&mut state, Message::ClearSelection, &ctx);

        assert!(state.selected.is_empty());
    }

    #[test]
    fn bulk_delete_removes_every_selected_unreferenced_tax_rate() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), direction: Some(TaxDirection::Output), ..Default::default() };
        state.name_input = "Sales Tax".into();
        state.rate_input = "15".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Service Tax".into();
        state.rate_input = "5".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.tax_rates.iter().map(|r| r.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.tax_rates.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes a tax rate still referenced by a posted
    /// tax transaction: the unreferenced one is deleted, the referenced
    /// one is skipped with a clear per-record error in the summary --
    /// bulk delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_tax_rate_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::tax::tax_transactions::TaxTransaction;
        use crate::backend::repositories::tax_repository::TaxRepository;

        let (ctx, gl_account) = ctx_with_a_gl_account();
        let accounting_repo = AccountingRepository::new(ctx.db.clone());
        accounting_repo.create_fiscal_period(&crate::backend::accounting::fiscal_periods::FiscalPeriod::new(ctx.company_id, "Open Period", chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap())).unwrap();
        let ar_account = accounting_repo.create_account(&Account::new(ctx.company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();

        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), direction: Some(TaxDirection::Output), ..Default::default() };
        state.name_input = "Sales Tax".into();
        state.rate_input = "15".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Service Tax".into();
        state.rate_input = "5".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.tax_rates.iter().find(|r| r.name == "Sales Tax").unwrap().id.unwrap();
        let free_id = state.tax_rates.iter().find(|r| r.name == "Service Tax").unwrap().id.unwrap();

        let tax = TaxRepository::new(ctx.db.clone());
        let tax_transaction = TaxTransaction::new(ctx.company_id, referenced_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(), rust_decimal_macros::dec!(1000), rust_decimal_macros::dec!(150), TaxDirection::Output);
        tax.post_tax(&tax_transaction, ar_account.id.unwrap()).unwrap();

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.tax_rates.len(), 1);
        assert_eq!(state.tax_rates[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_tax_rates() {
        let (ctx, gl_account) = ctx_with_temp_export_dir();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), direction: Some(TaxDirection::Output), ..Default::default() };
        state.name_input = "Sales Tax".into();
        state.rate_input = "15".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Service Tax".into();
        state.rate_input = "5".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.tax_rates.iter().find(|r| r.name == "Sales Tax").unwrap().id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("tax_rates_export.csv")).unwrap();
        assert!(contents.contains("Sales Tax") && !contents.contains("Service Tax"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
