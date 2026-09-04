//! Bank account summary: book balance and open items. Also supports
//! editing/deleting bank accounts (singly or in bulk) and exporting a
//! selection to CSV.

use std::collections::HashSet;

use chrono::Utc;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::banking::bank_accounts::BankAccount;
use crate::backend::core::validation::Validate;
use crate::backend::services::banking_service::BankingService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::toolbar::{self, StatusFilter};
use crate::frontend::components::common::{page_header, section_heading, side_panel};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

const COLUMN_LABELS: [&str; 3] = ["Bank", "Account #", "Book Balance"];
const DEFAULT_COLUMN_VISIBLE: [bool; 3] = [true, true, true];

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

#[derive(Debug, Clone, Default)]
pub struct State {
    bank_accounts: Vec<(BankAccount, String)>,
    gl_accounts: Vec<Choice>,
    gl_account: Option<Choice>,
    bank_name_input: String,
    account_number_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// bank account instead of creating a new one -- `Message::Create`
    /// does double duty as "save changes" in that state.
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
    GlAccountSelected(Choice),
    BankNameChanged(String),
    AccountNumberChanged(String),
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
    let service = BankingService::new(ctx.db.clone());
    let today = Utc::now().date_naive();

    let bank_accounts = service
        .list_bank_accounts(ctx.company_id)
        .map(|accounts| {
            accounts
                .into_iter()
                .map(|account| {
                    let balance = service.book_balance(account.id.unwrap_or_default(), today).map(|b| b.to_string()).unwrap_or_else(|_| "?".to_string());
                    (account, balance)
                })
                .collect()
        })
        .unwrap_or_default();

    let gl_accounts = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.account_code, a.account_name) }).collect())
        .unwrap_or_default();

    State {
        bank_accounts,
        gl_accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::GlAccountSelected(choice) => state.gl_account = Some(choice),
        Message::BankNameChanged(value) => state.bank_name_input = value,
        Message::AccountNumberChanged(value) => state.account_number_input = value,
        Message::Create => {
            state.message = None;
            state.error = None;
            let Some(gl_account) = &state.gl_account else {
                state.error = Some("choose a GL account".into());
                return;
            };

            if let Some(id) = state.editing_id {
                let Some((existing, balance)) = state.bank_accounts.iter().find(|(a, _)| a.id == Some(id)).cloned() else {
                    state.error = Some("that bank account no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let account = BankAccount { gl_account_id: gl_account.id, bank_name: state.bank_name_input.clone(), account_number: state.account_number_input.clone(), ..existing };
                if let Err(err) = account.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = BankingService::new(ctx.db.clone());
                match service.update_bank_account(&account) {
                    Ok(()) => {
                        if let Some(slot) = state.bank_accounts.iter_mut().find(|(a, _)| a.id == Some(id)) {
                            *slot = (account, balance);
                        }
                        state.editing_id = None;
                        state.bank_name_input.clear();
                        state.account_number_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let bank_account = BankAccount::new(ctx.company_id, gl_account.id, state.bank_name_input.clone(), state.account_number_input.clone(), "USD");
            let service = BankingService::new(ctx.db.clone());
            match service.create_bank_account(&bank_account) {
                Ok(created) => {
                    state.bank_accounts.push((created, "0".to_string()));
                    state.bank_name_input.clear();
                    state.account_number_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some((account, _)) = state.bank_accounts.iter().find(|(a, _)| a.id == Some(id)) {
                state.editing_id = Some(id);
                state.gl_account = state.gl_accounts.iter().find(|c| c.id == account.gl_account_id).cloned();
                state.bank_name_input = account.bank_name.clone();
                state.account_number_input = account.account_number.clone();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.gl_account = None;
            state.bank_name_input.clear();
            state.account_number_input.clear();
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
            let service = BankingService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_bank_account(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.bank_accounts.retain(|(a, _)| a.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("bank account #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} bank account(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let query = state.search.trim().to_lowercase();
            let selected: Vec<&(BankAccount, String)> = if state.selected.is_empty() {
                state.bank_accounts.iter().filter(|(a, _)| matches_filters(a, &query, state.status_filter)).collect()
            } else {
                state.bank_accounts.iter().filter(|(a, _)| state.selected.contains(&a.id.unwrap_or_default())).collect()
            };
            let headers = ["Bank", "Account #", "Book Balance", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|(a, balance)| vec![a.bank_name.clone(), a.account_number.clone(), balance.clone(), if a.is_active { "Active".to_string() } else { "Inactive".to_string() }])
                .collect();
            let path = ctx.export_path("bank_accounts_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} bank account(s) to {}", selected.len(), path.display())),
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

/// Whether a bank account passes both the search box and the status
/// filter -- shared by the table's live filtering in `view` and by
/// `BulkExportCsv`'s "no selection -> export everything currently
/// visible" fallback, so the two can never disagree.
fn matches_filters(account: &BankAccount, query: &str, status: StatusFilter) -> bool {
    status.matches(account.is_active) && (query.is_empty() || account.bank_name.to_lowercase().contains(query) || account.account_number.to_lowercase().contains(query))
}

const TITLE: &str = "Bank Accounts";
const DESCRIPTION: &str = "Every bank account on this company's books.";

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add bank account" };
    let header = container(page_header::view(theme, TITLE, DESCRIPTION, header_label, Message::Create)).padding(spacing::HEADER_PADDING);

    column![header, body(state, theme)].spacing(spacing::SPACE_20).into()
}

/// Same as [`view`], except the title sits inline with `switcher` (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) on the
/// title's own row instead of alone -- `wrap` converts this module's
/// own `Message` into whatever the caller's `switcher` produces, so the
/// two can share one `Element` tree despite having different message
/// types.
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(state: &'a State, theme: Theme, switcher: Element<'a, ParentMessage>, wrap: impl Fn(Message) -> ParentMessage + 'a) -> Element<'a, ParentMessage> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add bank account" };
    let header =
        container(page_header::page_header_view_with_switcher(theme, TITLE, DESCRIPTION, switcher, header_label, wrap(Message::Create))).padding(spacing::HEADER_PADDING);

    column![header, body(state, theme).map(wrap)].spacing(spacing::SPACE_20).into()
}

fn body(state: &State, theme: Theme) -> Element<'_, Message> {
    let toolbar = toolbar::list_toolbar(
        theme,
        &state.search,
        "Search bank accounts",
        Message::SearchChanged,
        state.status_filter,
        Message::StatusFilterChanged,
        Message::ResetAll,
        Message::ToggleColumns,
        Message::BulkExportCsv,
        !state.selected.is_empty(),
        Message::BulkDelete,
    );

    let heading = section_heading::view(theme, "Bank account list", "Book balance as of today for every account.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&(BankAccount, String)> = state.bank_accounts.iter().filter(|(account, _)| matches_filters(account, &query, state.status_filter)).collect();

    let columns = vec![
        ColumnDef::new("Bank", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Account #", Length::FillPortion(2)),
        ColumnDef::new("Book Balance", Length::FillPortion(1)),
    ];

    let rows = filtered.iter().map(|(account, balance)| Row::new(vec![Cell::text(account.bank_name.clone()), Cell::text(account.account_number.clone()), Cell::text(balance.clone())])).collect();

    let row_ids: Vec<i64> = filtered.iter().map(|(a, _)| a.id.unwrap_or_default()).collect();
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

    let form_heading = if state.editing_id.is_some() { "Edit bank account" } else { "Add a bank account" };
    let mut form = column![
        text(form_heading).size(typography::LABEL),
        pick_list(state.gl_accounts.as_slice(), state.gl_account.clone(), Message::GlAccountSelected).style(dropdown::style(theme)).placeholder("GL account"),
        text_input("Bank name", &state.bank_name_input).style(inputs::text(theme)).on_input(Message::BankNameChanged).width(Length::Fill).padding(spacing::SM),
        text_input("Account number", &state.account_number_input).style(inputs::text(theme)).on_input(Message::AccountNumberChanged).width(Length::Fill).padding(spacing::SM),
    ]
    .spacing(spacing::SM);

    if state.editing_id.is_some() {
        form = form.push(
            iced::widget::row![
                button(text("Save changes")).style(button_style::primary(theme)).on_press(Message::Create).padding(spacing::SM),
                button(text("Cancel")).style(button_style::secondary(theme)).on_press(Message::CancelEdit).padding(spacing::SM),
            ]
            .spacing(spacing::SM),
        );
    } else {
        form = form.push(button(text("Add bank account")).style(button_style::primary(theme)).on_press(Message::Create).padding(spacing::SM));
    }

    let mut content = column![toolbar].spacing(spacing::SPACE_20).padding(spacing::CONTENT_PADDING).width(Length::Fill);

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
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::banking_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_a_gl_account() -> (Ctx, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(banking_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from("."), uploads_dir: PathBuf::from(".") };
        let choice = Choice { id: cash.id.unwrap(), label: "1000 - Cash".into() };
        (ctx, choice)
    }

    fn ctx_with_temp_export_dir() -> (Ctx, Choice) {
        let (ctx, choice) = ctx_with_a_gl_account();
        (Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_bank_accounts_view_test_{:?}", std::thread::current().id())), ..ctx }, choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::GlAccountSelected(gl_account.clone()), &ctx);
        update(&mut state, Message::BankNameChanged("First National".into()), &ctx);
        update(&mut state, Message::AccountNumberChanged("12345".into()), &ctx);

        assert_eq!(state.gl_account, Some(gl_account));
        assert_eq!(state.bank_name_input, "First National");
        assert_eq!(state.account_number_input, "12345");
    }

    #[test]
    fn creating_without_a_gl_account_sets_an_error() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State { bank_name_input: "First National".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a GL account"));
        assert!(state.bank_accounts.is_empty());
    }

    #[test]
    fn creating_a_valid_bank_account_adds_it_with_a_zero_balance_and_clears_the_form() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            gl_account: Some(gl_account),
            bank_name_input: "First National".into(),
            account_number_input: "12345".into(),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.bank_accounts.len(), 1);
        assert_eq!(state.bank_accounts[0].0.bank_name, "First National");
        assert_eq!(state.bank_accounts[0].1, "0");
        assert!(state.bank_name_input.is_empty());
        assert!(state.account_number_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("first".into()), &ctx);

        assert_eq!(state.search, "first");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_bank_account() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), bank_name_input: "First National".into(), account_number_input: "12345".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.bank_accounts[0].0.id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.bank_name_input, "First National");
        assert_eq!(state.account_number_input, "12345");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_bank_account_in_place_and_clears_editing_state() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), bank_name_input: "First National".into(), account_number_input: "12345".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.bank_accounts[0].0.id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.bank_name_input = "Second National".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.bank_accounts.len(), 1);
        assert_eq!(state.bank_accounts[0].0.bank_name, "Second National");

        let service = BankingService::new(ctx.db.clone());
        let reloaded = service.list_bank_accounts(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].bank_name, "Second National");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_bank_account() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), bank_name_input: "First National".into(), account_number_input: "12345".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.bank_accounts[0].0.id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.bank_name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert_eq!(state.bank_accounts[0].0.bank_name, "First National");
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
    fn bulk_delete_removes_every_selected_unreferenced_bank_account() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), ..Default::default() };
        state.bank_name_input = "First National".into();
        state.account_number_input = "12345".into();
        update(&mut state, Message::Create, &ctx);
        state.bank_name_input = "Second National".into();
        state.account_number_input = "67890".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.bank_accounts.iter().map(|(a, _)| a.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.bank_accounts.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes a bank account still referenced by a
    /// bank transaction: the unreferenced one is deleted, the referenced
    /// one is skipped with a clear per-record error in the summary --
    /// bulk delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_bank_account_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::banking::bank_transactions::BankTransaction;
        use crate::backend::repositories::banking_repository::BankingRepository;

        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), ..Default::default() };
        state.bank_name_input = "First National".into();
        state.account_number_input = "12345".into();
        update(&mut state, Message::Create, &ctx);
        state.bank_name_input = "Second National".into();
        state.account_number_input = "67890".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.bank_accounts.iter().find(|(a, _)| a.bank_name == "First National").unwrap().0.id.unwrap();
        let free_id = state.bank_accounts.iter().find(|(a, _)| a.bank_name == "Second National").unwrap().0.id.unwrap();

        let banking = BankingRepository::new(ctx.db.clone());
        banking.create_bank_transaction(&BankTransaction::new(ctx.company_id, referenced_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(), "Customer payment", rust_decimal_macros::dec!(500))).unwrap();

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.bank_accounts.len(), 1);
        assert_eq!(state.bank_accounts[0].0.id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_bank_accounts() {
        let (ctx, gl_account) = ctx_with_temp_export_dir();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), ..Default::default() };
        state.bank_name_input = "First National".into();
        state.account_number_input = "12345".into();
        update(&mut state, Message::Create, &ctx);
        state.bank_name_input = "Second National".into();
        state.account_number_input = "67890".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.bank_accounts.iter().find(|(a, _)| a.bank_name == "First National").unwrap().0.id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("bank_accounts_export.csv")).unwrap();
        assert!(contents.contains("First National") && !contents.contains("Second National"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_selection_exports_every_currently_filtered_bank_account() {
        let (ctx, gl_account) = ctx_with_temp_export_dir();
        let mut state = State { gl_accounts: vec![gl_account.clone()], gl_account: Some(gl_account), ..Default::default() };
        state.bank_name_input = "First National".into();
        state.account_number_input = "12345".into();
        update(&mut state, Message::Create, &ctx);
        state.bank_name_input = "First Union".into();
        state.account_number_input = "22222".into();
        update(&mut state, Message::Create, &ctx);
        state.bank_name_input = "Second National".into();
        state.account_number_input = "67890".into();
        update(&mut state, Message::Create, &ctx);
        state.search = "first".into();

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("bank_accounts_export.csv")).unwrap();
        assert!(contents.contains("First National") && contents.contains("First Union"));
        assert!(!contents.contains("Second National"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_and_status_filter() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State { search: "first".into(), status_filter: StatusFilter::Inactive, ..Default::default() };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.status_filter, StatusFilter::All);
    }

    #[test]
    fn status_filter_narrows_to_only_matching_bank_accounts() {
        let mut active = BankAccount::new(1, 10, "First National", "12345", "USD");
        active.is_active = true;
        let mut inactive = BankAccount::new(1, 10, "Old Bank", "00000", "USD");
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
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::ColumnToggled(2), &ctx);
        assert_eq!(state.column_visible, vec![true, true, false]);

        update(&mut state, Message::ColumnToggled(2), &ctx);
        assert_eq!(state.column_visible, vec![true, true, true]);
    }

    #[test]
    fn select_all_deselect_all_and_reset_columns_set_the_expected_visibility() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
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
