use iced::widget::{
    button, column, container, pick_list, scrollable, text, text_editor, text_input, vertical_space,
};
use iced::{Element, Length};
use rust_decimal::prelude::ToPrimitive;
use std::collections::{HashMap, HashSet};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::{Account, STATUS_ACTIVE, STATUS_INACTIVE};
use crate::backend::core::reference_code;
use crate::backend::core::validation::Validate;
use crate::backend::services::accounting_service::AccountingService;
use crate::database::exports::csv;
use crate::frontend::components::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::toolbar::{self, StatusFilter};
use crate::frontend::components::{page_header, side_panel};
use crate::frontend::components::badges::BadgeTone;
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::dropdown;
use crate::frontend::components::fonts;
use crate::frontend::components::inputs;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
use crate::frontend::views::Ctx;

const ACCOUNT_TYPES: [AccountType; 5] = [
    AccountType::Asset,
    AccountType::Equity,
    AccountType::Liability,
    AccountType::Revenue,
    AccountType::Expense,
];

const COLUMN_LABELS: [&str; 12] = [
    "S.No",
    "Account Code",
    "Account Name",
    "Account Type",
    "Parent Account",
    "Debit",
    "Credit",
    "Net Balance",
    "Status",
    "Description",
    "Created",
    "Modified",
];
const DEFAULT_COLUMN_VISIBLE: [bool; 12] = [
    true, true, true, true, true, true, true, true, false, false, false, false,
];
const EXPORT_COLUMN_LABELS: [&str; 12] = [
    "S.No",
    "Account Code",
    "Account Name",
    "Account Type",
    "Parent Account",
    "Debit",
    "Credit",
    "Net Balance",
    "Status",
    "Description",
    "Created",
    "Modified",
];

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
    ParentChoice {
        id: None,
        label: "No parent (top-level)".to_string(),
    }
}

/// Every account as a possible parent, minus `excluding` (the account
/// currently being edited, if any) so an account can never be offered
/// as its own parent.
fn parent_choices(accounts: &[Account], excluding: Option<i64>) -> Vec<ParentChoice> {
    let mut choices = vec![no_parent_choice()];
    choices.extend(
        accounts
            .iter()
            .filter(|a| a.id != excluding)
            .map(|a| ParentChoice {
                id: a.id,
                label: format!("{} - {}", a.account_code, a.account_name),
            }),
    );
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
        if is_active {
            StatusChoice::Active
        } else {
            StatusChoice::Inactive
        }
    }
}

impl std::fmt::Display for StatusChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

const STATUS_CHOICES: [StatusChoice; 2] = [StatusChoice::Active, StatusChoice::Inactive];

fn format_timestamp(unix_seconds: i64) -> String {
    chrono::DateTime::from_timestamp(unix_seconds, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

#[derive(Debug, Default)]
pub struct State {
    accounts: Vec<Account>,
    balances: HashMap<i64, AccountBalance>,
    code_input: String,
    name_input: String,
    account_type: Option<AccountType>,
    parent_account_input: Option<ParentChoice>,
    description_input: String,
    description_content: text_editor::Content,
    status_input: StatusChoice,
    search: String,
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    add_panel_open: bool,
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
    DescriptionEdited(text_editor::Action),
    StatusSelected(StatusChoice),
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
        Err(err) => {
            return State {
                error: Some(err.to_string()),
                ..Default::default()
            }
        }
    };
    let balances = account_balances(&service, ctx.company_id);
    State {
        accounts,
        balances,
        ..Default::default()
    }
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
            (
                line.account_id,
                AccountBalance {
                    debit,
                    credit,
                    net_balance: debit - credit,
                },
            )
        })
        .collect()
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CodeChanged(value) => state.code_input = value,
        Message::NameChanged(value) => state.name_input = value,
        Message::AccountTypeChanged(account_type) => state.account_type = Some(account_type),
        Message::ParentAccountSelected(choice) => state.parent_account_input = Some(choice),
        Message::DescriptionChanged(value) => {
            state.description_input = value.clone();
            state.description_content = text_editor::Content::with_text(&value);
        }
        Message::DescriptionEdited(action) => {
            state.description_content.perform(action);
            state.description_input = state
                .description_content
                .text()
                .trim_end_matches('\n')
                .to_string();
        }
        Message::StatusSelected(choice) => state.status_input = choice,
        Message::Create => {
            state.message = None;
            state.error = None;
            let Some(account_type) = state.account_type else {
                state.error = Some("choose an account type".into());
                return;
            };
            let parent_account = state
                .parent_account_input
                .as_ref()
                .and_then(|choice| choice.id);

            if let Some(id) = state.editing_id {
                let Some(existing) = state
                    .accounts
                    .iter()
                    .find(|a: &&Account| a.id == Some(id))
                    .cloned()
                else {
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
                        let saved = Account {
                            modified_timestamp: chrono::Utc::now().timestamp(),
                            ..account
                        };
                        if let Some(slot) = state.accounts.iter_mut().find(|a| a.id == Some(id)) {
                            *slot = saved;
                        }
                        state
                            .accounts
                            .sort_by(|a, b| a.account_code.cmp(&b.account_code));
                        state.editing_id = None;
                        state.code_input.clear();
                        state.name_input.clear();
                        state.account_type = None;
                        state.parent_account_input = None;
                        state.description_input.clear();
                        state.description_content = text_editor::Content::new();
                        state.status_input = StatusChoice::Active;
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut account = Account::new(
                ctx.company_id,
                state.code_input.clone(),
                state.name_input.clone(),
                account_type,
            );
            account.parent_account = parent_account;
            account.description = state.description_input.clone();
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
                    state
                        .accounts
                        .sort_by(|a, b| a.account_code.cmp(&b.account_code));
                    state.code_input.clear();
                    state.name_input.clear();
                    state.account_type = None;
                    state.parent_account_input = None;
                    state.description_input.clear();
                    state.description_content = text_editor::Content::new();
                    state.status_input = StatusChoice::Active;
                    state.add_panel_open = false;
                    state.add_reference_code.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::OpenAddPanel => {
            state.message = None;
            state.error = None;
            state.editing_id = None;
            state.add_panel_open = true;
            state.columns_open = false;
            state.export_panel_open = false;
            state.add_reference_code = reference_code::generate_code("COA");
            state.code_input.clear();
            state.name_input.clear();
            state.account_type = None;
            state.parent_account_input = Some(no_parent_choice());
            state.description_input.clear();
            state.description_content = text_editor::Content::new();
            state.status_input = StatusChoice::Active;
        }
        Message::CloseAddPanel => {
            state.add_panel_open = false;
            state.editing_id = None;
            state.code_input.clear();
            state.name_input.clear();
            state.account_type = None;
            state.parent_account_input = None;
            state.description_input.clear();
            state.description_content = text_editor::Content::new();
            state.status_input = StatusChoice::Active;
            state.error = None;
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(account) = state.accounts.iter().find(|a| a.id == Some(id)) {
                state.editing_id = Some(id);
                state.add_panel_open = true;
                state.columns_open = false;
                state.export_panel_open = false;
                state.code_input = account.account_code.clone();
                state.name_input = account.account_name.clone();
                state.account_type = Some(account.account_type);
                state.parent_account_input = Some(
                    state
                        .accounts
                        .iter()
                        .find(|p| p.id == account.parent_account)
                        .map(|p| ParentChoice {
                            id: p.id,
                            label: format!("{} - {}", p.account_code, p.account_name),
                        })
                        .unwrap_or_else(no_parent_choice),
                );
                state.description_input = account.description.clone();
                state.description_content = text_editor::Content::with_text(&account.description);
                state.status_input = StatusChoice::from_is_active(account.is_active());
            }
        }
        Message::CancelEdit => {
            state.add_panel_open = false;
            state.editing_id = None;
            state.code_input.clear();
            state.name_input.clear();
            state.account_type = None;
            state.parent_account_input = None;
            state.description_input.clear();
            state.description_content = text_editor::Content::new();
            state.status_input = StatusChoice::Active;
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
                state.error = Some(format!(
                    "Deleted {deleted}; {} could not be deleted -- {}",
                    failures.len(),
                    failures.join("; ")
                ));
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
        Message::SelectAllExportColumns => {
            state.export_column_visible = vec![true; EXPORT_COLUMN_LABELS.len()]
        }
        Message::DeselectAllExportColumns => {
            state.export_column_visible = vec![false; EXPORT_COLUMN_LABELS.len()]
        }
        Message::ConfirmExport => {
            state.message = None;
            state.error = None;
            let query = state.search.trim().to_lowercase();
            let selected: Vec<&Account> = if state.selected.is_empty() {
                state
                    .accounts
                    .iter()
                    .filter(|a| matches_filters(a, &query, state.status_filter))
                    .collect()
            } else {
                state
                    .accounts
                    .iter()
                    .filter(|a| state.selected.contains(&a.id.unwrap_or_default()))
                    .collect()
            };
            let is_export_visible = |index: usize| {
                state
                    .export_column_visible
                    .get(index)
                    .copied()
                    .unwrap_or(true)
            };
            let headers: Vec<&str> = EXPORT_COLUMN_LABELS
                .iter()
                .enumerate()
                .filter(|(index, _)| is_export_visible(*index))
                .map(|(_, label)| *label)
                .collect();
            let rows: Vec<Vec<String>> = selected
                .iter()
                .enumerate()
                .map(|(row_index, a)| {
                    let balance =
                        a.id.and_then(|id| state.balances.get(&id))
                            .copied()
                            .unwrap_or_default();
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
                        if a.is_active() {
                            "Active".to_string()
                        } else {
                            "Inactive".to_string()
                        },
                        a.description.clone(),
                        format_timestamp(a.create_timestamp),
                        format_timestamp(a.modified_timestamp),
                    ];
                    fields
                        .into_iter()
                        .enumerate()
                        .filter(|(index, _)| is_export_visible(*index))
                        .map(|(_, field)| field)
                        .collect()
                })
                .collect();
            let path = ctx.export_path("chart_of_accounts_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => {
                    state.message = Some(format!(
                        "Exported {} account(s) to {}",
                        selected.len(),
                        path.display()
                    ));
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
    status.matches(account.is_active())
        && (query.is_empty()
            || account.account_name.to_lowercase().contains(query)
            || account.account_code.to_lowercase().contains(query))
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
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Element<'a, ParentMessage>,
    wrap: impl Fn(Message) -> ParentMessage + 'a,
) -> Element<'a, ParentMessage> {
    let header_label: &str = if state.add_panel_open {
        "Return"
    } else {
        "Add account"
    };
    let header_action: Message = if state.add_panel_open {
        Message::CloseAddPanel
    } else {
        Message::OpenAddPanel
    };
    let header = container(page_header::view_with_switcher_action_style(
        theme,
        TITLE,
        "",
        switcher,
        header_label,
        wrap(header_action),
        state.add_panel_open,
    ))
    .padding(spacing::HEADER_PADDING);
    column![header, body(state, theme).map(wrap)]
        .spacing(spacing::SPACE_4)
        .into()
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

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Account> = state
        .accounts
        .iter()
        .filter(|account| matches_filters(account, &query, state.status_filter))
        .collect();

    let columns: Vec<ColumnDef> = COLUMN_LABELS
        .iter()
        .enumerate()
        .map(|(index, &label)| {
            let width = match index {
                0 => Length::Fixed(70.0),
                1 => Length::Fixed(130.0),
                2 => Length::Fixed(260.0),
                3 => Length::Fixed(140.0),
                4 => Length::Fixed(220.0),
                5 => Length::Fixed(110.0),
                6 => Length::Fixed(110.0),
                7 => Length::Fixed(120.0),
                8 => Length::Fixed(110.0),
                9 => Length::Fixed(220.0),
                10 | 11 => Length::Fixed(160.0),
                _ => Length::Fixed(120.0),
            };
            let column = ColumnDef::new(label, width);
            if index == 1 {
                column.sorted(SortDirection::Ascending)
            } else {
                column
            }
        })
        .collect();
    let table_width = data_table::frozen_header_row_width(&columns, &state.column_visible, true);

    let rows = filtered
        .iter()
        .enumerate()
        .map(|(index, account)| {
            let balance = account
                .id
                .and_then(|id| state.balances.get(&id))
                .copied()
                .unwrap_or_default();
            Row::new(vec![
                Cell::text((index + 1).to_string()),
                Cell::text(account.account_code.clone()),
                Cell::text(account.account_name.clone()),
                Cell::text(account.account_type.as_str()),
                Cell::text(account_parent_label(&state.accounts, account)),
                Cell::text(format!("{:.2}", balance.debit)),
                Cell::text(format!("{:.2}", balance.credit)),
                Cell::text(format!("{:.2}", balance.net_balance)),
                if account.is_active() {
                    Cell::status("Active", BadgeTone::Positive)
                } else {
                    Cell::status("Inactive", BadgeTone::Neutral)
                },
                Cell::text(account.description.clone()),
                Cell::text(format_timestamp(account.create_timestamp)),
                Cell::text(format_timestamp(account.modified_timestamp)),
            ])
            .highlighted(state.editing_id.is_some() && account.id == state.editing_id)
        })
        .collect();

    let row_ids: Vec<i64> = filtered.iter().map(|a| a.id.unwrap_or_default()).collect();
    let select_ids = row_ids.clone();
    let selected_flags: Vec<bool> = row_ids
        .iter()
        .map(|id| state.selected.contains(id))
        .collect();
    let edit_ids = row_ids.clone();
    let edit_ids_primary = row_ids.clone();
    let table = data_table::view_with_frozen_header(
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
    // Fixed-width so the table keeps its original size (columns don't
    // compress) when the side panel narrows the space available to it
    // -- `scrollable` then grows a horizontal scrollbar for the part
    // that no longer fits, instead of the table shrinking to match.
    let primary_view: Element<'_, Message> = scrollable(
        container(table)
            .width(Length::Fixed(table_width))
            .height(Length::Shrink),
    )
    .direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::default(),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    let panel: Option<Element<'_, Message>> = if state.add_panel_open {
        Some(add_panel(state, theme))
    } else if state.columns_open {
        Some(data_table::columns_panel(
            theme,
            &COLUMN_LABELS,
            &state.column_visible,
            Message::ColumnToggled,
            Message::SelectAllColumns,
            Message::DeselectAllColumns,
            Message::ResetColumns,
        ))
    } else if state.export_panel_open {
        Some(export_panel(state, theme))
    } else {
        None
    };

    let mut table_content = column![]
        .spacing(spacing::SPACE_20)
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(message) = &state.message {
        table_content = table_content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        table_content = table_content.push(text(error.clone()).color(theme.negative_front()));
    }
    table_content = table_content.push(primary_view);

    let table_area: Element<'_, Message> = container(table_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    let below_toolbar: Element<'_, Message> = if let Some(panel) = panel {
        iced::widget::row![table_area, side_panel::card(theme, panel)]
            .spacing(spacing::SPACE_20)
            .height(Length::Fill)
            .into()
    } else {
        table_area
    };
    let content = column![toolbar, below_toolbar]
        .spacing(spacing::SPACE_12)
        .padding(spacing::CONTENT_PADDING)
        .width(Length::Fill)
        .height(Length::Fill);
    let base: Element<'_, Message> = content.into();

    side_panel::split(theme, base, None)
}

/// The "Add Account" panel's own content -- opened via
fn add_panel(state: &State, theme: Theme) -> Element<'_, Message> {
    let is_editing = state.editing_id.is_some();
    let code_value = if is_editing {
        &state.code_input
    } else {
        &state.add_reference_code
    };
    let cancel_message = if is_editing {
        Message::CancelEdit
    } else {
        Message::CloseAddPanel
    };

    column![
        iced::widget::row![
            container(
                text("Account Code")
                    .size(typography::body())
                    .color(theme.neutral_front())
            )
            .width(Length::FillPortion(3)),
            container(
                text_input("Account Code", code_value)
                    .style(inputs::text(theme))
                    .on_input_maybe(is_editing.then_some(Message::CodeChanged))
                    .width(Length::Fill)
                    .padding(spacing::SM)
            )
            .width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
        iced::widget::row![
            container(
                text("Account Name")
                    .size(typography::body())
                    .color(theme.neutral_front())
            )
            .width(Length::FillPortion(3)),
            container(
                text_input("Account Name", &state.name_input)
                    .style(inputs::text(theme))
                    .on_input(Message::NameChanged)
                    .width(Length::Fill)
                    .padding(spacing::SM)
            )
            .width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
        iced::widget::row![
            container(
                text("Account Type")
                    .size(typography::body())
                    .color(theme.neutral_front())
            )
            .width(Length::FillPortion(3)),
            container(
                pick_list(
                    ACCOUNT_TYPES,
                    state.account_type,
                    Message::AccountTypeChanged,
                )
                .style(dropdown::style(theme))
                .menu_style(dropdown::dropdown_menu(theme))
                .placeholder("Account Type")
                .width(Length::Fill),
            )
            .width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
        iced::widget::row![
            container(
                text("Parent Account")
                    .size(typography::body())
                    .color(theme.neutral_front())
            )
            .width(Length::FillPortion(3)),
            container(
                pick_list(
                    parent_choices(&state.accounts, state.editing_id,),
                    state.parent_account_input.clone(),
                    Message::ParentAccountSelected,
                )
                .style(dropdown::style(theme))
                .menu_style(dropdown::dropdown_menu(theme))
                .placeholder("Parent Account")
                .width(Length::Fill),
            )
            .width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
        iced::widget::row![
            container(
                text("Status")
                    .size(typography::body())
                    .color(theme.neutral_front())
            )
            .width(Length::FillPortion(3)),
            container(
                pick_list(
                    STATUS_CHOICES,
                    Some(state.status_input),
                    Message::StatusSelected,
                )
                .style(dropdown::style(theme))
                .menu_style(dropdown::dropdown_menu(theme))
                .placeholder("Status")
                .width(Length::Fill),
            )
            .width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
        iced::widget::row![
            container(
                text("Description")
                    .size(typography::body())
                    .color(theme.neutral_front())
            )
            .width(Length::FillPortion(3)),
            container(
                text_editor(&state.description_content)
                    .style(inputs::text_edit(theme))
                    .placeholder("Description")
                    .on_action(Message::DescriptionEdited)
                    .height(Length::Fixed(96.0))
                    .padding(spacing::SM),
            )
            .width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Start),
        vertical_space(),
        iced::widget::row![container(
            iced::widget::row![
                button(text("Save"))
                    .style(button_style::primary(theme))
                    .on_press(Message::Create)
                    .padding(spacing::SM),
                button(text("Cancel").font(fonts::BOLD))
                    .style(button_style::neutral(theme))
                    .on_press(cancel_message)
                    .padding(spacing::SM),
            ]
            .spacing(spacing::SM),
        )
        .width(Length::FillPortion(7)),]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(spacing::SPACE_12)
    .height(Length::Fill)
    .into()
}

/// The "Export" panel's own content -- opened via `side_panel::split`
fn export_panel(state: &State, theme: Theme) -> Element<'_, Message> {
    let select_all = button(
        text("Select All")
            .size(typography::body())
            .font(fonts::BOLD),
    )
    .padding([spacing::SM, spacing::SPACE_14])
    .style(button_style::neutral(theme))
    .on_press(Message::SelectAllExportColumns);
    let deselect_all = button(
        text("Deselect All")
            .size(typography::body())
            .font(fonts::BOLD),
    )
    .padding([spacing::SM, spacing::SPACE_14])
    .style(button_style::neutral(theme))
    .on_press(Message::DeselectAllExportColumns);

    let mut list = column![].spacing(spacing::SPACE_12);
    for (index, label) in EXPORT_COLUMN_LABELS.iter().enumerate() {
        let is_visible = state
            .export_column_visible
            .get(index)
            .copied()
            .unwrap_or(true);
        list = list.push(
            iced::widget::checkbox(*label, is_visible)
                .style(crate::frontend::components::checkbox::style(theme))
                .on_toggle(move |_| Message::ExportColumnToggled(index))
                .size(typography::checkbox_size())
                .text_size(typography::label()),
        );
    }

    let export_button = button(text("Export").size(typography::label()))
        .padding([spacing::SPACE_10, spacing::MD])
        .width(Length::Fill)
        .style(button_style::primary(theme))
        .on_press(Message::ConfirmExport);

    column![
        text("Select Columns to Export")
            .size(typography::panel_title())
            .color(theme.text_primary()),
        iced::widget::row![select_all, deselect_all].spacing(spacing::SM),
        scrollable(list).height(Length::Shrink),
        export_button,
    ]
    .spacing(spacing::MD)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Building the widget tree, not just running `update`, exercises
    /// the horizontally-scrolling table added for the frozen-column-width
    /// layout -- a `Scrollable` panics at construction (not later) if its
    /// content still fills the axis it scrolls, a class of bug `update`
    /// tests never touch. Covers both panel states since opening the add
    /// panel is what narrows the table's available space and is the
    /// scenario the horizontal scrollbar exists for.
    #[test]
    fn body_builds_without_panicking_with_and_without_the_side_panel() {
        let account = Account::new(1, "1000", "Assets", AccountType::Asset);
        let state = State {
            accounts: vec![account],
            ..Default::default()
        };
        let _: Element<'_, Message> = body(&state, Theme::Light);

        let state_with_panel = State {
            add_panel_open: true,
            ..state
        };
        let _: Element<'_, Message> = body(&state_with_panel, Theme::Dark);
    }
}
