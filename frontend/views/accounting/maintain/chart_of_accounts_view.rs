//! List the chart of accounts, add new accounts, edit or delete
//! existing ones (singly or in bulk), and export a selection to CSV.

use std::collections::{HashMap, HashSet};

use rust_decimal::prelude::ToPrimitive;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::{Account, STATUS_ACTIVE, STATUS_INACTIVE};
use crate::backend::core::reference_code;
use crate::backend::core::validation::Validate;
use crate::backend::services::accounting_service::AccountingService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::toolbar::{self, StatusFilter};
use crate::frontend::components::common::{page_header, section_heading, side_panel};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{fonts, spacing};
use crate::frontend::styles::typography;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

const ACCOUNT_TYPES: [AccountType; 5] = [
    AccountType::Asset,
    AccountType::Equity,
    AccountType::Liability,
    AccountType::Revenue,
    AccountType::Expense,
];

const COLUMN_LABELS: [&str; 12] = ["S.No", "Code", "Name", "Type", "Parent Account", "Debit", "Credit", "Net Balance", "Status", "Attachments", "Created", "Modified"];
/// Core columns default visible; the more detail-oriented ones (parent,
/// attachment count, timestamps) stay tucked behind the existing
/// columns panel rather than crowding the table on first load.
const DEFAULT_COLUMN_VISIBLE: [bool; 12] = [true, true, true, true, false, true, true, true, true, false, false, false];
const EXPORT_COLUMN_LABELS: [&str; 13] = ["S.No", "Code", "Name", "Type", "Parent Account", "Debit", "Credit", "Net Balance", "Status", "Description", "Attachments", "Created", "Modified"];

/// One account's debit/credit/net balance as of today, computed from
/// posted transactions the same way `trial_balance_view` does --
/// Chart of Accounts-specific display data, not part of `Account`
/// itself, so it's never written back through `AccountingService`.
#[derive(Debug, Clone, Copy, Default)]
struct AccountBalance {
    debit: f64,
    credit: f64,
    net_balance: f64,
}

/// One entry in the "Parent account" picker -- `id: None` is the
/// synthetic "No parent (top-level)" choice, always first, which is
/// also how a parent already set can be cleared back out.
#[derive(Debug, Clone, PartialEq)]
pub struct ParentChoice {
    id: Option<i64>,
    label: String,
}

impl std::fmt::Display for ParentChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn no_parent_choice() -> ParentChoice {
    ParentChoice { id: None, label: "No parent (top-level)".to_string() }
}

/// Every account as a possible parent, minus `excluding` (the account
/// currently being edited, if any) so an account can never be offered
/// as its own parent.
fn parent_choices(accounts: &[Account], excluding: Option<i64>) -> Vec<ParentChoice> {
    let mut choices = vec![no_parent_choice()];
    choices.extend(accounts.iter().filter(|a| a.id != excluding).map(|a| ParentChoice { id: a.id, label: format!("{} - {}", a.account_code, a.account_name) }));
    choices
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusChoice {
    #[default]
    Active,
    Inactive,
}

impl StatusChoice {
    fn as_str(self) -> &'static str {
        match self {
            StatusChoice::Active => STATUS_ACTIVE,
            StatusChoice::Inactive => STATUS_INACTIVE,
        }
    }

    fn from_is_active(is_active: bool) -> Self {
        if is_active { StatusChoice::Active } else { StatusChoice::Inactive }
    }
}

impl std::fmt::Display for StatusChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

const STATUS_CHOICES: [StatusChoice; 2] = [StatusChoice::Active, StatusChoice::Inactive];

fn format_timestamp(unix_seconds: i64) -> String {
    chrono::DateTime::from_timestamp(unix_seconds, 0).map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
}

#[derive(Debug, Clone, Default)]
pub struct State {
    accounts: Vec<Account>,
    /// Keyed by account id -- see [`AccountBalance`].
    balances: HashMap<i64, AccountBalance>,
    code_input: String,
    name_input: String,
    account_type: Option<AccountType>,
    parent_account_input: Option<ParentChoice>,
    description_input: String,
    status_input: StatusChoice,
    /// File names already copied into `Ctx::upload_path` for the
    /// account currently being added/edited -- becomes
    /// `Account::attachments` on save.
    attachment_input: Vec<String>,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// account instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    /// Whether the "Add Account" side panel is open.
    add_panel_open: bool,
    /// The reference code shown (read-only) in the open add panel --
    /// generated fresh the moment the panel opens and carried through
    /// unchanged into the saved account.
    add_reference_code: String,
    status_filter: StatusFilter,
    columns_open: bool,
    column_visible: Vec<bool>,
    export_panel_open: bool,
    export_column_visible: Vec<bool>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CodeChanged(String),
    NameChanged(String),
    AccountTypeChanged(AccountType),
    ParentAccountSelected(ParentChoice),
    DescriptionChanged(String),
    StatusSelected(StatusChoice),
    AttachFile,
    RemoveAttachment(usize),
    Create,
    OpenAddPanel,
    CloseAddPanel,
    EditRow(i64),
    CancelEdit,
    RowSelected(i64, bool),
    SelectAllChanged(Vec<i64>, bool),
    ClearSelection,
    BulkDelete,
    ToggleExportPanel,
    ExportColumnToggled(usize),
    SelectAllExportColumns,
    DeselectAllExportColumns,
    ConfirmExport,
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
    let service = AccountingService::new(ctx.db.clone());
    let accounts = match service.list_accounts(ctx.company_id) {
        Ok(accounts) => accounts,
        Err(err) => return State { error: Some(err.to_string()), ..Default::default() },
    };
    let balances = account_balances(&service, ctx.company_id);
    State { accounts, balances, ..Default::default() }
}

/// Every account's current debit/credit/net balance, derived from
/// posted transactions as of today the same way `trial_balance_view`
/// does. An account with no postings yet simply has no entry here --
/// callers default a missing lookup to zero, which is the correct
/// balance for it anyway.
fn account_balances(service: &AccountingService, company_id: i64) -> HashMap<i64, AccountBalance> {
    let today = chrono::Utc::now().date_naive();
    let Ok(trial_balance) = service.trial_balance_as_of(company_id, today) else {
        return HashMap::new();
    };
    trial_balance
        .lines
        .into_iter()
        .map(|line| {
            let debit = line.debit.to_f64().unwrap_or(0.0);
            let credit = line.credit.to_f64().unwrap_or(0.0);
            (line.account_id, AccountBalance { debit, credit, net_balance: debit - credit })
        })
        .collect()
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CodeChanged(value) => state.code_input = value,
        Message::NameChanged(value) => state.name_input = value,
        Message::AccountTypeChanged(account_type) => state.account_type = Some(account_type),
        Message::ParentAccountSelected(choice) => state.parent_account_input = Some(choice),
        Message::DescriptionChanged(value) => state.description_input = value,
        Message::StatusSelected(choice) => state.status_input = choice,
        Message::AttachFile => {
            state.error = None;
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                let Some(original_name) = path.file_name().and_then(|n| n.to_str()) else {
                    state.error = Some("that file has no usable name".into());
                    return;
                };
                // Prefixed with a fresh reference code so two attachments
                // (on the same account, or two different accounts) with
                // the same original file name never overwrite each
                // other's copy in the shared uploads directory.
                let stored_name = format!("{}_{original_name}", reference_code::generate_code("att"));
                match std::fs::copy(&path, ctx.upload_path(&stored_name)) {
                    Ok(_) => state.attachment_input.push(stored_name),
                    Err(err) => state.error = Some(err.to_string()),
                }
            }
        }
        Message::RemoveAttachment(index) => {
            if index < state.attachment_input.len() {
                let file_name = state.attachment_input.remove(index);
                let _ = std::fs::remove_file(ctx.upload_path(&file_name));
            }
        }
        Message::Create => {
            state.message = None;
            state.error = None;
            let Some(account_type) = state.account_type else {
                state.error = Some("choose an account type".into());
                return;
            };
            let parent_account = state.parent_account_input.as_ref().and_then(|choice| choice.id);

            if let Some(id) = state.editing_id {
                let Some(existing) = state.accounts.iter().find(|a: &&Account| a.id == Some(id)).cloned() else {
                    state.error = Some("that account no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let account = Account {
                    account_code: state.code_input.clone(),
                    account_name: state.name_input.clone(),
                    account_type,
                    parent_account,
                    status: state.status_input.as_str().to_string(),
                    description: state.description_input.clone(),
                    attachments: state.attachment_input.clone(),
                    ..existing
                };
                if let Err(err) = account.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = AccountingService::new(ctx.db.clone());
                match service.update_account(&account) {
                    Ok(()) => {
                        // Mirrors what the repository just stamped
                        // server-side, so the local echo shows the real
                        // modified time without a full reload.
                        let saved = Account { modified_timestamp: chrono::Utc::now().timestamp(), ..account };
                        if let Some(slot) = state.accounts.iter_mut().find(|a| a.id == Some(id)) {
                            *slot = saved;
                        }
                        state.accounts.sort_by(|a, b| a.account_code.cmp(&b.account_code));
                        state.editing_id = None;
                        state.code_input.clear();
                        state.name_input.clear();
                        state.account_type = None;
                        state.parent_account_input = None;
                        state.description_input.clear();
                        state.status_input = StatusChoice::Active;
                        state.attachment_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut account = Account::new(ctx.company_id, state.code_input.clone(), state.name_input.clone(), account_type);
            account.parent_account = parent_account;
            account.description = state.description_input.clone();
            account.attachments = state.attachment_input.clone();
            account.status = state.status_input.as_str().to_string();
            if !state.add_reference_code.is_empty() {
                account.reference_code = Some(state.add_reference_code.clone());
            }
            if let Err(err) = account.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let service = AccountingService::new(ctx.db.clone());
            match service.create_account(&account) {
                Ok(created) => {
                    state.accounts.push(created);
                    state.accounts.sort_by(|a, b| a.account_code.cmp(&b.account_code));
                    state.code_input.clear();
                    state.name_input.clear();
                    state.account_type = None;
                    state.parent_account_input = None;
                    state.description_input.clear();
                    state.status_input = StatusChoice::Active;
                    state.attachment_input.clear();
                    state.add_panel_open = false;
                    state.add_reference_code.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::OpenAddPanel => {
            state.message = None;
            state.error = None;
            state.add_panel_open = true;
            state.columns_open = false;
            state.export_panel_open = false;
            state.add_reference_code = reference_code::generate_code("coa");
            state.code_input.clear();
            state.name_input.clear();
            state.account_type = None;
            state.parent_account_input = Some(no_parent_choice());
            state.description_input.clear();
            state.status_input = StatusChoice::Active;
            state.attachment_input.clear();
        }
        Message::CloseAddPanel => {
            state.add_panel_open = false;
            state.code_input.clear();
            state.name_input.clear();
            state.account_type = None;
            state.parent_account_input = None;
            state.description_input.clear();
            state.status_input = StatusChoice::Active;
            state.attachment_input.clear();
            state.error = None;
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(account) = state.accounts.iter().find(|a| a.id == Some(id)) {
                state.editing_id = Some(id);
                state.code_input = account.account_code.clone();
                state.name_input = account.account_name.clone();
                state.account_type = Some(account.account_type);
                state.parent_account_input = Some(
                    state
                        .accounts
                        .iter()
                        .find(|p| p.id == account.parent_account)
                        .map(|p| ParentChoice { id: p.id, label: format!("{} - {}", p.account_code, p.account_name) })
                        .unwrap_or_else(no_parent_choice),
                );
                state.description_input = account.description.clone();
                state.status_input = StatusChoice::from_is_active(account.is_active());
                state.attachment_input = account.attachments.clone();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.code_input.clear();
            state.name_input.clear();
            state.account_type = None;
            state.parent_account_input = None;
            state.description_input.clear();
            state.status_input = StatusChoice::Active;
            state.attachment_input.clear();
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
            let service = AccountingService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_account(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.accounts.retain(|a| a.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("account #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} account(s)."));
            }
        }
        Message::ToggleExportPanel => {
            state.export_panel_open = !state.export_panel_open;
            if state.export_panel_open {
                state.add_panel_open = false;
                state.columns_open = false;
            }
        }
        Message::ExportColumnToggled(index) => {
            if index >= state.export_column_visible.len() {
                state.export_column_visible.resize(index + 1, true);
            }
            state.export_column_visible[index] = !state.export_column_visible[index];
        }
        Message::SelectAllExportColumns => state.export_column_visible = vec![true; EXPORT_COLUMN_LABELS.len()],
        Message::DeselectAllExportColumns => state.export_column_visible = vec![false; EXPORT_COLUMN_LABELS.len()],
        Message::ConfirmExport => {
            state.message = None;
            state.error = None;
            let query = state.search.trim().to_lowercase();
            let selected: Vec<&Account> = if state.selected.is_empty() {
                state.accounts.iter().filter(|a| matches_filters(a, &query, state.status_filter)).collect()
            } else {
                state.accounts.iter().filter(|a| state.selected.contains(&a.id.unwrap_or_default())).collect()
            };
            let is_export_visible = |index: usize| state.export_column_visible.get(index).copied().unwrap_or(true);
            let headers: Vec<&str> = EXPORT_COLUMN_LABELS.iter().enumerate().filter(|(index, _)| is_export_visible(*index)).map(|(_, label)| *label).collect();
            let rows: Vec<Vec<String>> = selected
                .iter()
                .enumerate()
                .map(|(row_index, a)| {
                    let balance = a.id.and_then(|id| state.balances.get(&id)).copied().unwrap_or_default();
                    let parent_label = account_parent_label(&state.accounts, a);
                    let fields = [
                        (row_index + 1).to_string(),
                        a.account_code.clone(),
                        a.account_name.clone(),
                        a.account_type.as_str().to_string(),
                        parent_label,
                        format!("{:.2}", balance.debit),
                        format!("{:.2}", balance.credit),
                        format!("{:.2}", balance.net_balance),
                        if a.is_active() { "Active".to_string() } else { "Inactive".to_string() },
                        a.description.clone(),
                        a.attachments.join("; "),
                        format_timestamp(a.create_timestamp),
                        format_timestamp(a.modified_timestamp),
                    ];
                    fields.into_iter().enumerate().filter(|(index, _)| is_export_visible(*index)).map(|(_, field)| field).collect()
                })
                .collect();
            let path = ctx.export_path("chart_of_accounts_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => {
                    state.message = Some(format!("Exported {} account(s) to {}", selected.len(), path.display()));
                    state.export_panel_open = false;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => state.search = value,
        Message::StatusFilterChanged(filter) => state.status_filter = filter,
        Message::ResetAll => {
            state.search.clear();
            state.status_filter = StatusFilter::All;
        }
        Message::ToggleColumns => {
            state.columns_open = !state.columns_open;
            if state.columns_open {
                state.add_panel_open = false;
                state.export_panel_open = false;
            }
        }
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

/// Whether an account passes both the search box and the status
/// filter -- shared by the table's live filtering in `view` and by
/// `ConfirmExport`'s "no selection -> export everything currently
/// visible" fallback, so the two can never disagree.
fn matches_filters(account: &Account, query: &str, status: StatusFilter) -> bool {
    status.matches(account.is_active()) && (query.is_empty() || account.account_name.to_lowercase().contains(query) || account.account_code.to_lowercase().contains(query))
}

/// `account`'s parent, rendered as "Code - Name", or "-" for a
/// top-level account -- shared by the table's Parent Account column
/// and CSV export.
fn account_parent_label(accounts: &[Account], account: &Account) -> String {
    account
        .parent_account
        .and_then(|parent_id| accounts.iter().find(|a| a.id == Some(parent_id)))
        .map(|parent| format!("{} - {}", parent.account_code, parent.account_name))
        .unwrap_or_else(|| "-".to_string())
}

const TITLE: &str = "Chart of Accounts";
const DESCRIPTION: &str = "Every general ledger account on this company's books.";

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add account" };
    let header_action = if state.editing_id.is_some() { Message::Create } else { Message::OpenAddPanel };
    let header = container(page_header::view(theme, TITLE, DESCRIPTION, header_label, header_action)).padding(spacing::HEADER_PADDING);
    column![header, body(state, theme)].spacing(spacing::SPACE_20).into()
}

pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(state: &'a State, theme: Theme, switcher: Element<'a, ParentMessage>, wrap: impl Fn(Message) -> ParentMessage + 'a) -> Element<'a, ParentMessage> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add account" };
    let header_action = if state.editing_id.is_some() { Message::Create } else { Message::OpenAddPanel };
    let header =
        container(page_header::page_header_view_with_switcher(theme, TITLE, DESCRIPTION, switcher, header_label, wrap(header_action))).padding(spacing::HEADER_PADDING);

    column![header, body(state, theme).map(wrap)].spacing(spacing::SPACE_20).into()
}

fn body(state: &State, theme: Theme) -> Element<'_, Message> {
    let toolbar = toolbar::list_toolbar(
        theme,
        &state.search,
        "Search accounts",
        Message::SearchChanged,
        state.status_filter,
        Message::StatusFilterChanged,
        Message::ResetAll,
        Message::ToggleColumns,
        Message::ToggleExportPanel,
        !state.selected.is_empty(),
        Message::BulkDelete,
    );

    let heading = section_heading::view(theme, "Account list", "Every account, grouped by code order.");
    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Account> = state.accounts.iter().filter(|account| matches_filters(account, &query, state.status_filter)).collect();

    let columns = vec![
        ColumnDef::new("S.No", Length::FillPortion(1)),
        ColumnDef::new("Code", Length::FillPortion(1)).sorted(SortDirection::Ascending),
        ColumnDef::new("Name", Length::FillPortion(3)),
        ColumnDef::new("Type", Length::FillPortion(1)),
        ColumnDef::new("Parent Account", Length::FillPortion(2)),
        ColumnDef::new("Debit", Length::FillPortion(1)),
        ColumnDef::new("Credit", Length::FillPortion(1)),
        ColumnDef::new("Net Balance", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
        ColumnDef::new("Attachments", Length::FillPortion(1)),
        ColumnDef::new("Created", Length::FillPortion(2)),
        ColumnDef::new("Modified", Length::FillPortion(2)),
    ];

    let rows = filtered
        .iter()
        .enumerate()
        .map(|(index, account)| {
            let balance = account.id.and_then(|id| state.balances.get(&id)).copied().unwrap_or_default();
            Row::new(vec![
                Cell::text((index + 1).to_string()),
                Cell::text(account.account_code.clone()),
                Cell::text(account.account_name.clone()),
                Cell::text(account.account_type.as_str()),
                Cell::text(account_parent_label(&state.accounts, account)),
                Cell::text(format!("{:.2}", balance.debit)),
                Cell::text(format!("{:.2}", balance.credit)),
                Cell::text(format!("{:.2}", balance.net_balance)),
                if account.is_active() { Cell::status("Active", BadgeTone::Positive) } else { Cell::status("Inactive", BadgeTone::Neutral) },
                Cell::text(account.attachments.len().to_string()),
                Cell::text(format_timestamp(account.create_timestamp)),
                Cell::text(format_timestamp(account.modified_timestamp)),
            ])
        })
        .collect();
    let row_ids: Vec<i64> = filtered.iter().map(|a| a.id.unwrap_or_default()).collect();
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

    let mut content = column![toolbar].spacing(spacing::SPACE_20).padding(spacing::CONTENT_PADDING).width(Length::Fill);

    content = content.push(heading).push(table);

    // Editing stays an inline form below the table, unchanged -- only
    // adding a new account moved to the side panel.
    if state.editing_id.is_some() {
        let form = column![
            text("Edit account").size(typography::LABEL),
            text_input("Code", &state.code_input).style(inputs::text(theme)).on_input(Message::CodeChanged).width(Length::Fill).padding(spacing::SM),
            text_input("Name", &state.name_input).style(inputs::text(theme)).on_input(Message::NameChanged).width(Length::Fill).padding(spacing::SM),
            pick_list(ACCOUNT_TYPES, state.account_type, Message::AccountTypeChanged).style(dropdown::style(theme)).placeholder("Type"),
            pick_list(parent_choices(&state.accounts, state.editing_id), state.parent_account_input.clone(), Message::ParentAccountSelected).style(dropdown::style(theme)).placeholder("Parent account"),
            pick_list(STATUS_CHOICES, Some(state.status_input), Message::StatusSelected).style(dropdown::style(theme)).placeholder("Status"),
            text_input("Description", &state.description_input).style(inputs::text(theme)).on_input(Message::DescriptionChanged).width(Length::Fill).padding(spacing::SM),
            column![text("Attachments").size(typography::BODY).color(theme.neutral_front()), attachment_list(state, theme), attach_file_button(theme)].spacing(spacing::XS),
            iced::widget::row![
                button(text("Save changes").size(fonts::BUTTON_FONT_SIZE)).style(button_style::primary(theme)).on_press(Message::Create).padding(spacing::SM),
                button(text("Cancel").size(fonts::BUTTON_FONT_SIZE)).style(button_style::neutral(theme)).on_press(Message::CancelEdit).padding(spacing::SM),
            ]
            .spacing(spacing::SM),
        ]
        .spacing(spacing::SM);
        content = content.push(form);
    }

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    let base: Element<'_, Message> = container(scrollable(content).height(Length::Fill)).width(Length::Fill).into();

    let panel: Option<Element<'_, Message>> = if state.add_panel_open {
        Some(add_panel(state, theme))
    } else if state.columns_open {
        Some(data_table::columns_panel(theme, &COLUMN_LABELS, &state.column_visible, Message::ColumnToggled, Message::SelectAllColumns, Message::DeselectAllColumns, Message::ResetColumns))
    } else if state.export_panel_open {
        Some(export_panel(state, theme))
    } else {
        None
    };

    side_panel::split(theme, base, panel)
}

/// The list of already-attached file names for the account currently
/// being added/edited, each with its own "Remove" button -- shared by
/// [`add_panel`] and the inline edit form in [`body`].
fn attachment_list(state: &State, theme: Theme) -> Element<'_, Message> {
    let mut list = column![].spacing(spacing::XS);
    for (index, file_name) in state.attachment_input.iter().enumerate() {
        list = list.push(
            iced::widget::row![
                text(file_name.clone()).size(typography::BODY),
                button(text("Remove").size(typography::BODY)).style(button_style::reset(theme)).on_press(Message::RemoveAttachment(index)).padding(spacing::XS),
            ]
            .spacing(spacing::SM)
            .align_y(iced::Alignment::Center),
        );
    }
    list.into()
}

fn attach_file_button(theme: Theme) -> Element<'static, Message> {
    button(text("Attach file")).style(button_style::neutral(theme)).on_press(Message::AttachFile).padding(spacing::SM).into()
}

/// The "Add Account" panel's own content -- opened via
/// `side_panel::split`, which supplies the side-by-side layout and
/// card chrome around whatever this returns.
fn add_panel(state: &State, theme: Theme) -> Element<'_, Message> {
    column![
        text("Add an account").size(typography::PANEL_TITLE),
        column![text("Reference Code").size(typography::BODY).color(theme.neutral_front()), text(state.add_reference_code.clone()).size(typography::EMPHASIS)].spacing(spacing::XS),
        column![
            text("Account Number").size(typography::BODY).color(theme.neutral_front()),
            text_input("e.g. 1000", &state.code_input).style(inputs::text(theme)).on_input(Message::CodeChanged).width(Length::Fill).padding(spacing::SM),
        ]
        .spacing(spacing::XS),
        text_input("Name", &state.name_input).style(inputs::text(theme)).on_input(Message::NameChanged).width(Length::Fill).padding(spacing::SM),
        pick_list(ACCOUNT_TYPES, state.account_type, Message::AccountTypeChanged).style(dropdown::style(theme)).placeholder("Type"),
        pick_list(parent_choices(&state.accounts, state.editing_id), state.parent_account_input.clone(), Message::ParentAccountSelected).style(dropdown::style(theme)).placeholder("Parent account"),
        pick_list(STATUS_CHOICES, Some(state.status_input), Message::StatusSelected).style(dropdown::style(theme)).placeholder("Status"),
        text_input("Description", &state.description_input).style(inputs::text(theme)).on_input(Message::DescriptionChanged).width(Length::Fill).padding(spacing::SM),
        column![text("Attachments").size(typography::BODY).color(theme.neutral_front()), attachment_list(state, theme), attach_file_button(theme)].spacing(spacing::XS),
        iced::widget::row![
            button(text("Save")).style(button_style::primary(theme)).on_press(Message::Create).padding(spacing::SM),
            button(text("Cancel")).style(button_style::neutral(theme)).on_press(Message::CloseAddPanel).padding(spacing::SM),
        ]
        .spacing(spacing::SM),
    ]
    .spacing(spacing::SPACE_12)
    .into()
}

/// The "Export" panel's own content -- opened via `side_panel::split`
/// the same way [`add_panel`] and `data_table::columns_panel` are.
/// Mirrors `columns_panel`'s "Select Columns to Display" shape but for
/// "Select Columns to Export": a checklist (defaulting to all-checked,
/// independent of the table's own `column_visible`) plus a bottom
/// "Export" button that fires the actual write via `Message::ConfirmExport`.
fn export_panel(state: &State, theme: Theme) -> Element<'_, Message> {
    let select_all = button(text("Select All").size(typography::BODY)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::reset(theme)).on_press(Message::SelectAllExportColumns);
    let deselect_all = button(text("Deselect All").size(typography::BODY)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::reset(theme)).on_press(Message::DeselectAllExportColumns);

    let mut list = column![].spacing(spacing::SPACE_12);
    for (index, label) in EXPORT_COLUMN_LABELS.iter().enumerate() {
        let is_visible = state.export_column_visible.get(index).copied().unwrap_or(true);
        list = list.push(iced::widget::checkbox(*label, is_visible).style(crate::frontend::styles::checkbox::style(theme)).on_toggle(move |_| Message::ExportColumnToggled(index)).size(16).text_size(typography::LABEL));
    }

    let export_button = button(text("Export").size(typography::LABEL)).padding([spacing::SPACE_10, spacing::MD]).width(Length::Fill).style(button_style::primary(theme)).on_press(Message::ConfirmExport);

    column![
        text("Select Columns to Export").size(typography::PANEL_TITLE).color(theme.text_primary()),
        iced::widget::row![select_all, deselect_all].spacing(spacing::SM),
        scrollable(list).height(Length::Fill),
        export_button,
    ]
    .spacing(spacing::MD)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from("."), uploads_dir: PathBuf::from(".") }
    }

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_chart_of_accounts_view_test_{:?}", std::thread::current().id())), ..ctx }
    }

    fn create_basic_account(state: &mut State, ctx: &Ctx, code: &str, name: &str, account_type: AccountType) {
        state.code_input = code.into();
        state.name_input = name.into();
        state.account_type = Some(account_type);
        update(state, Message::Create, ctx);
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::CodeChanged("1000".into()), &ctx);
        update(&mut state, Message::NameChanged("Cash".into()), &ctx);
        update(&mut state, Message::AccountTypeChanged(AccountType::Asset), &ctx);
        update(&mut state, Message::DescriptionChanged("Petty cash drawer".into()), &ctx);

        assert_eq!(state.code_input, "1000");
        assert_eq!(state.name_input, "Cash");
        assert_eq!(state.account_type, Some(AccountType::Asset));
        assert_eq!(state.description_input, "Petty cash drawer");
    }

    #[test]
    fn creating_without_choosing_a_type_sets_an_error() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose an account type"));
        assert!(state.accounts.is_empty());
    }

    #[test]
    fn creating_a_valid_account_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);

        assert!(state.error.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].account_code, "1000");
        assert!(state.accounts[0].is_active());
        assert_eq!(state.accounts[0].create_timestamp, state.accounts[0].modified_timestamp);
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
        assert!(state.account_type.is_none());
    }

    #[test]
    fn creating_with_a_parent_and_description_persists_both() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Assets", AccountType::Asset);
        let parent_id = state.accounts[0].id.unwrap();

        state.code_input = "1010".into();
        state.name_input = "Cash".into();
        state.account_type = Some(AccountType::Asset);
        state.parent_account_input = Some(ParentChoice { id: Some(parent_id), label: "1000 - Assets".into() });
        state.description_input = "Operating cash".into();
        update(&mut state, Message::Create, &ctx);

        let child = state.accounts.iter().find(|a| a.account_code == "1010").unwrap();
        assert_eq!(child.parent_account, Some(parent_id));
        assert_eq!(child.description, "Operating cash");
    }

    #[test]
    fn attach_and_remove_attachment_update_the_pending_list() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        state.attachment_input.push("coa-20260101000000000_invoice.pdf".into());
        state.attachment_input.push("coa-20260101000000001_receipt.pdf".into());
        update(&mut state, Message::RemoveAttachment(0), &ctx);

        assert_eq!(state.attachment_input, vec!["coa-20260101000000001_receipt.pdf".to_string()]);
    }

    #[test]
    fn status_selected_updates_the_pending_status() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        assert_eq!(state.status_input, StatusChoice::Active);

        update(&mut state, Message::StatusSelected(StatusChoice::Inactive), &ctx);

        assert_eq!(state.status_input, StatusChoice::Inactive);
    }

    #[test]
    fn creating_as_inactive_is_reflected_in_is_active() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), account_type: Some(AccountType::Asset), status_input: StatusChoice::Inactive, ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(!state.accounts[0].is_active());
        assert_eq!(state.accounts[0].status, STATUS_INACTIVE);
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("cash".into()), &ctx);

        assert_eq!(state.search, "cash");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_account() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        let id = state.accounts[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.code_input, "1000");
        assert_eq!(state.name_input, "Cash");
        assert_eq!(state.account_type, Some(AccountType::Asset));
        assert_eq!(state.parent_account_input, Some(no_parent_choice()));
        assert_eq!(state.status_input, StatusChoice::Active);
    }

    #[test]
    fn saving_an_edit_updates_the_existing_account_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        let id = state.accounts[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Petty Cash".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].account_name, "Petty Cash");

        let service = AccountingService::new(ctx.db.clone());
        let reloaded = service.list_accounts(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].account_name, "Petty Cash");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_account() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        let id = state.accounts[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.accounts[0].account_name, "Cash");
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
    fn bulk_delete_removes_every_selected_unreferenced_account() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        create_basic_account(&mut state, &ctx, "1010", "Petty Cash", AccountType::Asset);
        let ids: Vec<i64> = state.accounts.iter().map(|a| a.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.accounts.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes an account still referenced by a
    /// posted transaction: the unreferenced one is deleted, the
    /// referenced one is skipped with a clear per-record error in the
    /// summary -- bulk delete does not block the whole batch over one
    /// stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_account_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::transactions::transaction::Transaction;
        use crate::backend::transactions::transaction_lines::TransactionLine;
        use crate::backend::transactions::transaction_types::TransactionType;

        let ctx = ctx_with_company();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        create_basic_account(&mut state, &ctx, "3000", "Owner's Capital", AccountType::Equity);
        create_basic_account(&mut state, &ctx, "1010", "Petty Cash", AccountType::Asset);

        let cash_id = state.accounts.iter().find(|a| a.account_code == "1000").unwrap().id.unwrap();
        let capital_id = state.accounts.iter().find(|a| a.account_code == "3000").unwrap().id.unwrap();
        let free_id = state.accounts.iter().find(|a| a.account_code == "1010").unwrap().id.unwrap();

        let service = AccountingService::new(ctx.db.clone());
        service
            .create_fiscal_period(&crate::backend::accounting::fiscal_periods::FiscalPeriod::new(
                ctx.company_id,
                "Open Period",
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
            ))
            .unwrap();
        service
            .post_transaction(
                &Transaction::new(ctx.company_id, TransactionType::OpeningBalance, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
                    .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(1000)))
                    .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(1000))),
            )
            .unwrap();

        state.selected = HashSet::from([cash_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.accounts.len(), 2);
        assert!(state.accounts.iter().any(|a| a.id == Some(cash_id)));
        assert!(!state.accounts.iter().any(|a| a.id == Some(free_id)));
        assert_eq!(state.selected, HashSet::from([cash_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn open_add_panel_shows_it_with_a_fresh_reference_code_and_a_cleared_form() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "stale".into(), name_input: "Stale".into(), account_type: Some(AccountType::Asset), ..Default::default() };

        update(&mut state, Message::OpenAddPanel, &ctx);

        assert!(state.add_panel_open);
        assert!(state.add_reference_code.starts_with("coa-"));
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
        assert!(state.account_type.is_none());
        assert_eq!(state.parent_account_input, Some(no_parent_choice()));
        assert_eq!(state.status_input, StatusChoice::Active);
    }

    #[test]
    fn close_add_panel_hides_it_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::OpenAddPanel, &ctx);
        state.code_input = "1000".into();
        state.name_input = "Cash".into();

        update(&mut state, Message::CloseAddPanel, &ctx);

        assert!(!state.add_panel_open);
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
    }

    #[test]
    fn reopening_the_panel_after_cancel_generates_a_fresh_code_not_the_cancelled_one() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::OpenAddPanel, &ctx);
        let first_code = state.add_reference_code.clone();
        update(&mut state, Message::CloseAddPanel, &ctx);
        update(&mut state, Message::OpenAddPanel, &ctx);

        assert_ne!(state.add_reference_code, first_code, "a cancelled code must never be reused");
        assert!(state.add_reference_code.starts_with("coa-"));
    }

    #[test]
    fn creating_via_the_panel_saves_the_generated_code_the_entered_name_and_the_selected_type() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::OpenAddPanel, &ctx);
        let shown_reference_code = state.add_reference_code.clone();

        update(&mut state, Message::CodeChanged("1000".into()), &ctx);
        update(&mut state, Message::NameChanged("Cash".into()), &ctx);
        update(&mut state, Message::AccountTypeChanged(AccountType::Asset), &ctx);
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(!state.add_panel_open, "saving must close the panel");
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].account_code, "1000");
        assert_eq!(state.accounts[0].account_name, "Cash");
        assert_eq!(state.accounts[0].account_type, AccountType::Asset);
        assert_eq!(state.accounts[0].reference_code.as_deref(), Some(shown_reference_code.as_str()), "the saved reference code must match exactly what the panel showed");

        // Persisted, not just held in the in-memory list.
        let service = AccountingService::new(ctx.db.clone());
        let reloaded = service.list_accounts(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].reference_code.as_deref(), Some(shown_reference_code.as_str()));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_accounts() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        create_basic_account(&mut state, &ctx, "1010", "Petty Cash", AccountType::Asset);
        let export_id = state.accounts.iter().find(|a| a.account_code == "1000").unwrap().id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::ConfirmExport, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("chart_of_accounts_export.csv")).unwrap();
        assert!(contents.contains("Cash") && !contents.contains("Petty Cash"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_selection_exports_every_currently_filtered_account() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        create_basic_account(&mut state, &ctx, "1000", "Cash", AccountType::Asset);
        create_basic_account(&mut state, &ctx, "1010", "Petty Cash", AccountType::Asset);
        create_basic_account(&mut state, &ctx, "2000", "Accounts Payable", AccountType::Liability);
        state.search = "cash".into();

        update(&mut state, Message::ConfirmExport, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("chart_of_accounts_export.csv")).unwrap();
        assert!(contents.contains("Cash") && contents.contains("Petty Cash"));
        assert!(!contents.contains("Accounts Payable"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_and_status_filter() {
        let ctx = ctx_with_company();
        let mut state = State { search: "cash".into(), status_filter: StatusFilter::Inactive, ..Default::default() };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.status_filter, StatusFilter::All);
    }

    #[test]
    fn status_filter_narrows_to_only_matching_accounts() {
        let account = Account::new(1, "1000", "Cash", AccountType::Asset);
        let mut inactive = Account::new(1, "1010", "Old Account", AccountType::Asset);
        inactive.set_active(false);

        assert!(matches_filters(&account, "", StatusFilter::All));
        assert!(matches_filters(&account, "", StatusFilter::Active));
        assert!(!matches_filters(&account, "", StatusFilter::Inactive));

        assert!(matches_filters(&inactive, "", StatusFilter::All));
        assert!(!matches_filters(&inactive, "", StatusFilter::Active));
        assert!(matches_filters(&inactive, "", StatusFilter::Inactive));
    }

    #[test]
    fn account_parent_label_reports_a_dash_for_a_top_level_account_and_code_dash_name_otherwise() {
        let parent = Account { id: Some(1), ..Account::new(1, "1000", "Assets", AccountType::Asset) };
        let mut child = Account::new(1, "1010", "Cash", AccountType::Asset);
        child.parent_account = Some(1);

        assert_eq!(account_parent_label(&[parent.clone()], &parent), "-");
        assert_eq!(account_parent_label(&[parent], &child), "1000 - Assets");
    }

    #[test]
    fn column_toggled_flips_that_columns_visibility() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::ColumnToggled(1), &ctx);
        assert_eq!(state.column_visible[1], false);

        update(&mut state, Message::ColumnToggled(1), &ctx);
        assert_eq!(state.column_visible[1], true);
    }

    #[test]
    fn select_all_deselect_all_and_reset_columns_set_the_expected_visibility() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::DeselectAllColumns, &ctx);
        assert_eq!(state.column_visible, vec![false; COLUMN_LABELS.len()]);

        update(&mut state, Message::ColumnToggled(0), &ctx);
        update(&mut state, Message::ResetColumns, &ctx);
        assert_eq!(state.column_visible, DEFAULT_COLUMN_VISIBLE.to_vec());

        update(&mut state, Message::DeselectAllColumns, &ctx);
        update(&mut state, Message::SelectAllColumns, &ctx);
        assert_eq!(state.column_visible, vec![true; COLUMN_LABELS.len()]);
    }
}
