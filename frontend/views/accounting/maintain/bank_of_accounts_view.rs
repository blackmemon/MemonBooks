//! Bank Accounts: a responsive grid of account cards with computed
//! balances, an add/edit side panel, and a full-page detail view listing
//! every posted transaction that touches the account. Balances are never
//! typed: they come from the account's opening balance plus its ledger
//! (see `backend/banking/bank_ledger.rs` for the sign convention).

use chrono::NaiveDate;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, responsive, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use rust_decimal::Decimal;

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::banking::bank_accounts::{BankAccount, BankAccountType, BANK_ACCOUNT_TYPES};
use crate::backend::banking::bank_ledger::{BankBalances, BankLedger, BankLedgerEntry};
use crate::backend::core::validation::Validate;
use crate::backend::repositories::banking_repository::Removal;
use crate::backend::services::banking_service::BankingService;
use crate::database::exports::csv;
use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::labels::Side;
use crate::frontend::components::row_snap::{self, RowSnap};
use crate::frontend::components::table as table_style;
use crate::frontend::components::toolbar::{self, StatusFilter};
use crate::frontend::components::{
    dropdown, forms, headers, icons, labels, page_header, pagination, panels, scrollbar, search,
    spacing, states, typography,
};
use crate::frontend::views::Ctx;

/// Every field of a bank account and of its balances, in display order.
/// This is the only place a field's name is written: everything else
/// refers to a field by its index, through the `F_*` constants below.
const BANK_FIELDS: [&str; 13] = [
    "Bank Name",
    "Account Number",
    "Parent Company",
    "Sub Company",
    "Type",
    "Opening Balance",
    "Ledger Account",
    "Credit Balance",
    "Debit Balance",
    "Net Balance",
    "Last Transaction",
    "Running Balance",
    "Status",
];
const F_BANK_NAME: usize = 0;
const F_ACCOUNT_NUMBER: usize = 1;
const F_PARENT_COMPANY: usize = 2;
const F_SUB_COMPANY: usize = 3;
const F_TYPE: usize = 4;
const F_OPENING_BALANCE: usize = 5;
const F_LEDGER_ACCOUNT: usize = 6;
const F_CREDIT_BALANCE: usize = 7;
const F_DEBIT_BALANCE: usize = 8;
const F_NET_BALANCE: usize = 9;
const F_LAST_TRANSACTION: usize = 10;
const F_RUNNING_BALANCE: usize = 11;
const F_STATUS: usize = 12;

/// The fields written to the CSV export, in order.
const EXPORT_FIELDS: [usize; 12] = [
    F_BANK_NAME,
    F_ACCOUNT_NUMBER,
    F_PARENT_COMPANY,
    F_SUB_COMPANY,
    F_TYPE,
    F_LEDGER_ACCOUNT,
    F_OPENING_BALANCE,
    F_CREDIT_BALANCE,
    F_DEBIT_BALANCE,
    F_NET_BALANCE,
    F_LAST_TRANSACTION,
    F_STATUS,
];

const TITLE: &str = "Bank Accounts";
const ADD_LABEL: &str = "Add Account";
const ACTIVE_LABEL: &str = "Active";
const ARCHIVED_LABEL: &str = "Archived";
const ALL_LABEL: &str = "All";
const EMPTY_DASH: &str = "--";
/// Used only when the company record cannot be read.
const FALLBACK_CURRENCY: &str = "USD";
/// Cards per page: divisible by every grid width (1, 2 and 3 columns).
const CARD_PAGE_SIZES: [usize; 3] = [12, 24, 48];
const TRANSACTION_PAGE_SIZES: [usize; 3] = [25, 50, 100];
const EXPORT_FILE: &str = "bank_accounts_export.csv";

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

/// One card: the account and its balances, computed from its ledger.
#[derive(Debug, Clone)]
struct AccountSummary {
    account: BankAccount,
    balances: BankBalances,
    last_transaction: Option<NaiveDate>,
}

impl AccountSummary {
    fn new(account: BankAccount, ledger: &BankLedger) -> Self {
        Self {
            balances: ledger.balances(),
            last_transaction: ledger.last_transaction_date(),
            account,
        }
    }

    fn id(&self) -> i64 {
        self.account.id.unwrap_or_default()
    }
}

/// The add/edit panel's inputs, kept as typed.
#[derive(Debug, Clone, Default)]
struct AccountForm {
    bank_name: String,
    account_number: String,
    parent_company: String,
    sub_company: String,
    account_type: BankAccountType,
    opening_balance: String,
    gl_account: Option<Choice>,
}

/// The full-page view of one account.
#[derive(Debug, Clone)]
struct Detail {
    account_id: i64,
    ledger: BankLedger,
    error: Option<String>,
    search: String,
    oldest_first: bool,
    page: usize,
    page_size: usize,
    /// The ledger entry (by index) whose lines are shown.
    expanded: Option<usize>,
}

impl Detail {
    fn new(account_id: i64) -> Self {
        Self {
            account_id,
            ledger: BankLedger::default(),
            error: None,
            search: String::new(),
            oldest_first: false,
            page: 0,
            page_size: TRANSACTION_PAGE_SIZES[0],
            expanded: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    accounts: Vec<AccountSummary>,
    gl_accounts: Vec<Choice>,
    currency: String,
    search: String,
    /// `None` shows every type.
    type_filter: Option<BankAccountType>,
    status_filter: StatusFilter,
    page: usize,
    page_size: usize,
    /// Which row of cards the grid starts at.
    scroll: RowSnap,
    form_open: bool,
    /// `Some(id)` while the panel edits an existing account instead of
    /// adding one -- `Message::Create` then saves the changes.
    editing_id: Option<i64>,
    form: AccountForm,
    form_error: Option<String>,
    detail: Option<Detail>,
    /// The account a Delete press is waiting to have confirmed.
    pending_removal: Option<i64>,
    message: Option<String>,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            gl_accounts: Vec::new(),
            currency: FALLBACK_CURRENCY.into(),
            search: String::new(),
            type_filter: None,
            status_filter: StatusFilter::All,
            page: 0,
            page_size: CARD_PAGE_SIZES[0],
            scroll: RowSnap::default(),
            form_open: false,
            editing_id: None,
            form: AccountForm::default(),
            form_error: None,
            detail: None,
            pending_removal: None,
            message: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenForm,
    GlAccountSelected(Choice),
    BankNameChanged(String),
    AccountNumberChanged(String),
    ParentCompanyChanged(String),
    SubCompanyChanged(String),
    TypeChanged(BankAccountType),
    OpeningBalanceChanged(String),
    /// Saves the panel: adds an account, or saves the one being edited.
    Create,
    EditRow(i64),
    CancelEdit,
    Remove(i64),
    ConfirmRemove,
    CancelRemove,
    SearchChanged(String),
    TypeFilterChanged(Option<BankAccountType>),
    StatusFilterChanged(StatusFilter),
    ResetAll,
    BulkExportCsv,
    GoToPage(usize),
    PrevPage,
    PageSizeChanged(usize),
    OpenDetail(i64),
    CloseDetail,
    DetailSearchChanged(String),
    ToggleOrder,
    ToggleInfo(usize),
    DetailGoToPage(usize),
    DetailPrevPage,
    DetailPageSizeChanged(usize),
    /// The card grid scrolled by a row, from the wheel or its bar.
    Scrolled(row_snap::Scroll),
}

pub fn load(ctx: &Ctx) -> State {
    let service = BankingService::new(ctx.db.clone());
    let gl_accounts = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| {
            accounts
                .iter()
                .map(|a| Choice {
                    id: a.id.unwrap_or_default(),
                    label: format!("{} - {}", a.account_code, a.account_name),
                })
                .collect()
        })
        .unwrap_or_default();
    let mut state = State {
        gl_accounts,
        currency: service
            .company_currency(ctx.company_id)
            .unwrap_or_else(|_| FALLBACK_CURRENCY.into()),
        ..State::default()
    };
    refresh_accounts(&mut state, ctx);
    state
}

/// Reloads every account and recomputes its balances.
fn refresh_accounts(state: &mut State, ctx: &Ctx) {
    let service = BankingService::new(ctx.db.clone());
    match service.list_bank_accounts(ctx.company_id) {
        Ok(accounts) => {
            state.accounts = accounts
                .into_iter()
                .map(|account| {
                    let ledger = service
                        .bank_ledger_summary(account.id.unwrap_or_default())
                        .unwrap_or_default();
                    AccountSummary::new(account, &ledger)
                })
                .collect();
        }
        Err(err) => state.error = Some(err.to_string()),
    }
}

/// Messages that change which cards the grid shows, so it starts over
/// at the first row.
fn restarts_scroll(message: &Message) -> bool {
    matches!(
        message,
        Message::SearchChanged(_)
            | Message::TypeFilterChanged(_)
            | Message::StatusFilterChanged(_)
            | Message::ResetAll
            | Message::GoToPage(_)
            | Message::PrevPage
            | Message::PageSizeChanged(_)
    )
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    if restarts_scroll(&message) {
        state.scroll.reset();
    }
    match message {
        Message::Scrolled(scroll) => state.scroll.scroll(scroll),
        Message::OpenForm => open_form(state, None),
        Message::GlAccountSelected(choice) => state.form.gl_account = Some(choice),
        Message::BankNameChanged(value) => state.form.bank_name = value,
        Message::AccountNumberChanged(value) => state.form.account_number = value,
        Message::ParentCompanyChanged(value) => state.form.parent_company = value,
        Message::SubCompanyChanged(value) => state.form.sub_company = value,
        Message::TypeChanged(kind) => state.form.account_type = kind,
        Message::OpeningBalanceChanged(value) => state.form.opening_balance = value,
        Message::Create => save(state, ctx),
        Message::EditRow(id) => open_form(state, Some(id)),
        Message::CancelEdit => close_form(state),
        Message::Remove(id) => {
            state.message = None;
            state.error = None;
            state.pending_removal = Some(id);
        }
        Message::ConfirmRemove => confirm_removal(state, ctx),
        Message::CancelRemove => state.pending_removal = None,
        Message::SearchChanged(value) => {
            state.search = value;
            state.page = 0;
        }
        Message::TypeFilterChanged(kind) => {
            state.type_filter = kind;
            state.page = 0;
        }
        Message::StatusFilterChanged(filter) => {
            state.status_filter = filter;
            state.page = 0;
        }
        Message::ResetAll => {
            state.search.clear();
            state.type_filter = None;
            state.status_filter = StatusFilter::All;
            state.page = 0;
        }
        Message::BulkExportCsv => export_csv(state, ctx),
        Message::GoToPage(page) => state.page = page,
        Message::PrevPage => state.page = state.page.saturating_sub(1),
        Message::PageSizeChanged(size) => {
            state.page_size = size;
            state.page = 0;
        }
        Message::OpenDetail(id) => open_detail(state, id, ctx),
        Message::CloseDetail => state.detail = None,
        Message::DetailSearchChanged(value) => {
            if let Some(detail) = &mut state.detail {
                detail.search = value;
                detail.page = 0;
            }
        }
        Message::ToggleOrder => {
            if let Some(detail) = &mut state.detail {
                detail.oldest_first = !detail.oldest_first;
                detail.page = 0;
            }
        }
        Message::ToggleInfo(index) => {
            if let Some(detail) = &mut state.detail {
                detail.expanded = (detail.expanded != Some(index)).then_some(index);
            }
        }
        Message::DetailGoToPage(page) => {
            if let Some(detail) = &mut state.detail {
                detail.page = page;
            }
        }
        Message::DetailPrevPage => {
            if let Some(detail) = &mut state.detail {
                detail.page = detail.page.saturating_sub(1);
            }
        }
        Message::DetailPageSizeChanged(size) => {
            if let Some(detail) = &mut state.detail {
                detail.page_size = size;
                detail.page = 0;
            }
        }
    }
}

fn find(state: &State, id: i64) -> Option<&AccountSummary> {
    state
        .accounts
        .iter()
        .find(|summary| summary.account.id == Some(id))
}

/// Opens the panel empty (`None`) or prefilled from an account.
fn open_form(state: &mut State, id: Option<i64>) {
    state.message = None;
    state.error = None;
    state.form_error = None;
    let account = id.and_then(|id| find(state, id)).map(|s| s.account.clone());
    state.form = match &account {
        Some(account) => AccountForm {
            bank_name: account.bank_name.clone(),
            account_number: account.account_number.clone(),
            parent_company: account.parent_company.clone(),
            sub_company: account.sub_company.clone(),
            account_type: account.account_type,
            opening_balance: account.opening_balance.to_string(),
            gl_account: state
                .gl_accounts
                .iter()
                .find(|choice| choice.id == account.gl_account_id)
                .cloned(),
        },
        None => AccountForm::default(),
    };
    state.editing_id = account.and_then(|account| account.id);
    state.form_open = true;
}

fn close_form(state: &mut State) {
    state.form_open = false;
    state.editing_id = None;
    state.form = AccountForm::default();
    state.form_error = None;
}

/// The typed opening balance: blank is zero, thousands separators are
/// ignored, anything else must be a number.
fn parse_amount(value: &str) -> Option<Decimal> {
    let cleaned: String = value
        .chars()
        .filter(|c| *c != ',' && !c.is_whitespace())
        .collect();
    if cleaned.is_empty() {
        return Some(Decimal::ZERO);
    }
    cleaned.parse().ok()
}

/// The panel's account, or the first problem with what was typed.
fn form_account(state: &State) -> Result<BankAccount, String> {
    let form = &state.form;
    let required = |field: usize, value: &str| {
        if value.trim().is_empty() {
            Err(format!("{} is required.", BANK_FIELDS[field]))
        } else {
            Ok(())
        }
    };
    required(F_BANK_NAME, &form.bank_name)?;
    required(F_ACCOUNT_NUMBER, &form.account_number)?;
    let Some(gl_account) = &form.gl_account else {
        return Err("choose a GL account".into());
    };
    let Some(opening_balance) = parse_amount(&form.opening_balance) else {
        return Err(format!(
            "{} must be a number.",
            BANK_FIELDS[F_OPENING_BALANCE]
        ));
    };
    let existing = state
        .editing_id
        .and_then(|id| find(state, id))
        .map(|summary| summary.account.clone());
    let base = existing.unwrap_or_else(|| {
        BankAccount::new(
            0,
            gl_account.id,
            String::new(),
            String::new(),
            state.currency.clone(),
        )
    });
    Ok(BankAccount {
        gl_account_id: gl_account.id,
        bank_name: form.bank_name.trim().to_string(),
        account_number: form.account_number.trim().to_string(),
        parent_company: form.parent_company.trim().to_string(),
        sub_company: form.sub_company.trim().to_string(),
        account_type: form.account_type,
        opening_balance,
        ..base
    })
}

fn save(state: &mut State, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    state.form_error = None;
    let account = match form_account(state) {
        Ok(account) => BankAccount {
            company_id: ctx.company_id,
            ..account
        },
        Err(problem) => {
            state.form_error = Some(problem);
            return;
        }
    };
    if let Err(err) = account.validate() {
        state.form_error = Some(err.to_string());
        return;
    }
    let service = BankingService::new(ctx.db.clone());
    let result = if account.id.is_some() {
        service
            .update_bank_account(&account)
            .map(|()| "Saved changes.")
    } else {
        service
            .create_bank_account(&account)
            .map(|_| "Added bank account.")
    };
    match result {
        Ok(done) => {
            refresh_accounts(state, ctx);
            if let Some(id) = state.detail.as_ref().map(|detail| detail.account_id) {
                load_ledger(state, id, ctx);
            }
            close_form(state);
            state.message = Some(done.into());
        }
        Err(err) => state.form_error = Some(err.to_string()),
    }
}

/// Deletes the pending account, or archives it when it has history.
fn confirm_removal(state: &mut State, ctx: &Ctx) {
    let Some(id) = state.pending_removal.take() else {
        return;
    };
    let name = find(state, id)
        .map(|summary| summary.account.bank_name.clone())
        .unwrap_or_default();
    let service = BankingService::new(ctx.db.clone());
    match service.remove_bank_account(id) {
        Ok(Removal::Deleted) => {
            if state.detail.as_ref().map(|d| d.account_id) == Some(id) {
                state.detail = None;
            }
            if state.editing_id == Some(id) {
                close_form(state);
            }
            state.message = Some(format!("Deleted {name}."));
        }
        Ok(Removal::Archived) => {
            state.message = Some(format!(
                "{name} has transactions, so it was archived instead of deleted."
            ));
        }
        Err(err) => state.error = Some(err.to_string()),
    }
    refresh_accounts(state, ctx);
}

fn open_detail(state: &mut State, id: i64, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    state.pending_removal = None;
    state.detail = Some(Detail::new(id));
    load_ledger(state, id, ctx);
}

/// Loads (or reloads) the open detail's ledger, keeping its filters.
fn load_ledger(state: &mut State, id: i64, ctx: &Ctx) {
    let ledger = BankingService::new(ctx.db.clone()).bank_ledger(id);
    if let Some(detail) = state.detail.as_mut().filter(|d| d.account_id == id) {
        match ledger {
            Ok(ledger) => {
                detail.ledger = ledger;
                detail.error = None;
            }
            Err(err) => detail.error = Some(err.to_string()),
        }
        detail.expanded = None;
    }
}

/// Whether an account passes the search box, type and status filters --
/// shared by the card grid and the CSV export so the two never disagree.
fn matches_filters(
    account: &BankAccount,
    query: &str,
    kind: Option<BankAccountType>,
    status: StatusFilter,
) -> bool {
    status.matches(account.is_active)
        && kind.is_none_or(|kind| account.account_type == kind)
        && (query.is_empty()
            || [
                &account.bank_name,
                &account.account_number,
                &account.parent_company,
                &account.sub_company,
            ]
            .iter()
            .any(|value| value.to_lowercase().contains(query)))
}

fn filtered(state: &State) -> Vec<&AccountSummary> {
    let query = state.search.trim().to_lowercase();
    state
        .accounts
        .iter()
        .filter(|s| matches_filters(&s.account, &query, state.type_filter, state.status_filter))
        .collect()
}

fn export_csv(state: &mut State, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    let rows: Vec<Vec<String>> = filtered(state)
        .iter()
        .map(|summary| {
            EXPORT_FIELDS
                .iter()
                .map(|&f| field_text(state, summary, f))
                .collect()
        })
        .collect();
    let headers: Vec<&str> = EXPORT_FIELDS.iter().map(|&f| BANK_FIELDS[f]).collect();
    let path = ctx.export_path(EXPORT_FILE);
    match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
        Ok(()) => {
            state.message = Some(format!(
                "Exported {} bank account(s) to {}",
                rows.len(),
                path.display()
            ))
        }
        Err(err) => state.error = Some(err.to_string()),
    }
}

/// One field of an account as plain text.
fn field_text(state: &State, summary: &AccountSummary, field: usize) -> String {
    let account = &summary.account;
    let balances = &summary.balances;
    match field {
        F_BANK_NAME => account.bank_name.clone(),
        F_ACCOUNT_NUMBER => account.account_number.clone(),
        F_PARENT_COMPANY => account.parent_company.clone(),
        F_SUB_COMPANY => account.sub_company.clone(),
        F_TYPE => account.account_type.to_string(),
        F_LEDGER_ACCOUNT => state
            .gl_accounts
            .iter()
            .find(|choice| choice.id == account.gl_account_id)
            .map(|choice| choice.label.clone())
            .unwrap_or_default(),
        F_OPENING_BALANCE => labels::money(balances.opening),
        F_CREDIT_BALANCE => labels::money(balances.credit),
        F_DEBIT_BALANCE => labels::money(balances.debit),
        F_NET_BALANCE => labels::money(balances.net()),
        F_LAST_TRANSACTION => summary.last_transaction.map(day).unwrap_or_default(),
        F_STATUS => status_label(account.is_active).into(),
        _ => String::new(),
    }
}

fn status_label(is_active: bool) -> &'static str {
    if is_active {
        ACTIVE_LABEL
    } else {
        ARCHIVED_LABEL
    }
}

fn day(date: NaiveDate) -> String {
    date.format("%d %b %Y").to_string()
}

fn dash(value: &str) -> String {
    if value.trim().is_empty() {
        EMPTY_DASH.into()
    } else {
        value.to_string()
    }
}

/// The side of the books an account type sits on.
fn side_of(kind: BankAccountType) -> Side {
    match kind {
        BankAccountType::Debit => Side::Debit,
        BankAccountType::Credit => Side::Credit,
    }
}

/// The pill and card tint of an account type, and the color of its net
/// balance: its side's tone.
fn type_tone(kind: BankAccountType) -> BadgeTone {
    labels::side_tone(side_of(kind))
}

/// The side a ledger entry posts to: credit when it carries a credit
/// and no debit.
fn entry_side(entry: &BankLedgerEntry) -> Side {
    if entry.debit.is_zero() && !entry.credit.is_zero() {
        Side::Credit
    } else {
        Side::Debit
    }
}

/// `Parent · Sub`, skipping whichever is blank.
fn companies_line(account: &BankAccount) -> String {
    let names: Vec<&str> = [&account.parent_company, &account.sub_company]
        .into_iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect();
    if names.is_empty() {
        EMPTY_DASH.into()
    } else {
        names.join("  ·  ")
    }
}

/// A page of filtered items and the numbers behind its footer.
struct Paged<T> {
    rows: Vec<T>,
    total: usize,
    pages: usize,
    page: usize,
}

fn paginate<T>(items: Vec<T>, page: usize, size: usize) -> Paged<T> {
    let total = items.len();
    let pages = total.div_ceil(size.max(1)).max(1);
    let page = page.min(pages - 1);
    let rows = items.into_iter().skip(page * size).take(size).collect();
    Paged {
        rows,
        total,
        pages,
        page,
    }
}

// -- view ------------------------------------------------------------------

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    layout(state, theme, None, |message| message)
}

/// Same as [`view`], except the title sits inline with `switcher` (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) -- `wrap`
/// converts this module's own `Message` into whatever the caller's
/// `switcher` produces, so the two can share one `Element` tree despite
/// having different message types.
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Element<'a, ParentMessage>,
    wrap: impl Fn(Message) -> ParentMessage + 'a,
) -> Element<'a, ParentMessage> {
    layout(state, theme, Some(switcher), wrap)
}

/// The header (or, on the detail page, just the switcher) above the
/// width-dependent content, which is rebuilt whenever the size changes.
fn layout<'a, M: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Option<Element<'a, M>>,
    wrap: impl Fn(Message) -> M + 'a,
) -> Element<'a, M> {
    let mut page = column![]
        .spacing(spacing::SPACE_20)
        .padding(spacing::SPACE_20)
        .width(Length::Fill)
        .height(Length::Fill);
    if state.detail.is_some() {
        if let Some(switcher) = switcher {
            page = page.push(switcher);
        }
    } else {
        page = page.push(header(theme, switcher, wrap(Message::OpenForm)));
    }
    let body: Element<'a, Message> =
        responsive(move |size| content(state, theme, size.width)).into();
    page.push(body.map(wrap)).into()
}

/// Title (with the switcher beside it) on the left, Add Account on the right.
fn header<'a, M: Clone + 'a>(
    theme: Theme,
    switcher: Option<Element<'a, M>>,
    on_add: M,
) -> Element<'a, M> {
    let mut titles =
        row![labels::strong(theme, TITLE, headers::page_title_size()).wrapping(Wrapping::None)]
            .spacing(spacing::SPACE_12)
            .align_y(Alignment::Center);
    if let Some(switcher) = switcher {
        titles = titles.push(switcher);
    }
    row![
        container(titles).width(Length::Fill).clip(true),
        add_button(theme, on_add),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center)
    .into()
}

fn add_button<'a, M: Clone + 'a>(theme: Theme, on_press: M) -> Element<'a, M> {
    button_style::icon_label(icons::ADD, ADD_LABEL, [spacing::SPACE_10, spacing::MD])
        .style(button_style::mode(theme, true))
        .width(Length::Shrink)
        .on_press(on_press)
        .into()
}

/// The list or detail page beside (or, when narrow, replaced by) the
/// add/edit panel, for a content area `width` wide.
fn content(state: &State, theme: Theme, width: f32) -> Element<'_, Message> {
    let split = spacing::panel_split(width, state.form_open, spacing::SPACE_20);
    let mut main = row![].spacing(spacing::SPACE_20).height(Length::Fill);
    if let Some(content_width) = split.content {
        let page = match state
            .detail
            .as_ref()
            .and_then(|detail| find(state, detail.account_id).map(|summary| (detail, summary)))
        {
            Some((detail, summary)) => detail_page(state, theme, detail, summary, content_width),
            None => account_list(state, theme, content_width),
        };
        main = main.push(
            container(page)
                .width(Length::Fixed(content_width))
                .height(Length::Fill),
        );
    }
    if let Some(panel_width) = split.panel {
        main = main.push(form_panel(
            state,
            theme,
            spacing::layout_class(width),
            panel_width,
        ));
    }
    main.into()
}

/// Inline notices: the delete confirmation, the last result, an error.
fn notices(state: &State, theme: Theme) -> Option<Element<'_, Message>> {
    let mut items = column![].spacing(spacing::SPACE_8);
    let mut any = false;
    if let Some(summary) = state.pending_removal.and_then(|id| find(state, id)) {
        any = true;
        items = items.push(panels::padded(
            theme,
            row![
                text(format!(
                    "Delete {}? An account that has transactions is archived instead.",
                    summary.account.bank_name
                ))
                .size(typography::label())
                .color(theme.warning_front())
                .width(Length::Fill),
                button_style::bold("Delete", [spacing::SPACE_6, spacing::SPACE_14])
                    .style(button_style::destructive(theme))
                    .on_press(Message::ConfirmRemove),
                button_style::bold("Keep", [spacing::SPACE_6, spacing::SPACE_14])
                    .style(button_style::outlined(theme, false))
                    .on_press(Message::CancelRemove),
            ]
            .spacing(spacing::SPACE_8)
            .align_y(Alignment::Center),
        ));
    }
    if let Some(message) = &state.message {
        any = true;
        items = items.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        any = true;
        items = items.push(text(error.clone()).color(theme.negative_front()));
    }
    any.then(|| items.into())
}

/// Search, type and status filters, Reset and Export.
fn list_toolbar(state: &State, theme: Theme, class: spacing::LayoutClass) -> Element<'_, Message> {
    let type_count = |kind: Option<BankAccountType>| {
        state
            .accounts
            .iter()
            .filter(|s| kind.is_none_or(|kind| s.account.account_type == kind))
            .count()
    };
    let mut type_options = vec![(
        ALL_LABEL,
        type_count(None),
        Message::TypeFilterChanged(None),
        state.type_filter.is_none(),
    )];
    for kind in BANK_ACCOUNT_TYPES {
        type_options.push((
            kind.as_str(),
            type_count(Some(kind)),
            Message::TypeFilterChanged(Some(kind)),
            state.type_filter == Some(kind),
        ));
    }
    let active = state
        .accounts
        .iter()
        .filter(|s| s.account.is_active)
        .count();
    let status_options = vec![
        (
            ALL_LABEL,
            state.accounts.len(),
            Message::StatusFilterChanged(StatusFilter::All),
            state.status_filter == StatusFilter::All,
        ),
        (
            ACTIVE_LABEL,
            active,
            Message::StatusFilterChanged(StatusFilter::Active),
            state.status_filter == StatusFilter::Active,
        ),
        (
            ARCHIVED_LABEL,
            state.accounts.len() - active,
            Message::StatusFilterChanged(StatusFilter::Inactive),
            state.status_filter == StatusFilter::Inactive,
        ),
    ];
    let filters = row![
        dropdown::counted_filter(theme, BANK_FIELDS[F_TYPE], type_options),
        dropdown::counted_filter(theme, BANK_FIELDS[F_STATUS], status_options),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let tools = row![
        button_style::icon_tool(theme, icons::RESET, "Reset", Message::ResetAll, true),
        button_style::icon_tool(
            theme,
            icons::EXPORT,
            "Export CSV",
            Message::BulkExportCsv,
            true
        ),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    toolbar::filter_bar(
        class,
        search::input(
            theme,
            "Search bank, account number or company",
            &state.search,
            Message::SearchChanged,
        ),
        filters.into(),
        tools.into(),
    )
}

/// Toolbar, notices, the card grid (or an empty state) and its pager.
fn account_list(state: &State, theme: Theme, width: f32) -> Element<'_, Message> {
    let class = spacing::layout_class(width);
    let cards = paginate(filtered(state), state.page, state.page_size);
    let mut list = column![list_toolbar(state, theme, class)]
        .spacing(spacing::SPACE_20)
        .height(Length::Fill);
    if let Some(notices) = notices(state, theme) {
        list = list.push(notices);
    }
    if state.accounts.is_empty() {
        return list
            .push(panels::empty_state(
                theme,
                icons::LANDMARK,
                "No bank accounts yet",
                Some("Add the company's first bank account to start tracking its balance."),
                Some(add_button(theme, Message::OpenForm)),
            ))
            .into();
    }
    if cards.rows.is_empty() {
        return list
            .push(panels::empty_state(
                theme,
                icons::SEARCH,
                "No bank accounts match these filters",
                None,
                Some(
                    button_style::bold("Reset", [spacing::SPACE_6, spacing::SPACE_14])
                        .style(button_style::outlined(theme, false))
                        .on_press(Message::ResetAll)
                        .into(),
                ),
            ))
            .into();
    }
    let shown = cards.rows.len();
    let rows = cards.rows;
    let (page, pages, total, page_size) = (cards.page, cards.pages, cards.total, state.page_size);
    list = list.push(responsive(move |size| {
        let reserve = if pages > 1 {
            pagination::footer_height(class)
        } else {
            0.0
        };
        let mut area = column![card_grid(
            state,
            theme,
            &rows,
            spacing::grid_columns(class),
            size.height - reserve,
        )];
        if pages > 1 {
            area = area.push(pagination::footer(pagination::Footer {
                theme,
                class,
                page,
                pages,
                shown,
                total,
                page_size,
                page_sizes: &CARD_PAGE_SIZES,
                on_previous: Message::PrevPage,
                on_page: Message::GoToPage,
                on_page_size: Message::PageSizeChanged,
            }));
        }
        area.into()
    }));
    list.into()
}

/// The cards in `height` pixels, scrolling by whole rows.
fn card_grid<'a>(
    state: &'a State,
    theme: Theme,
    rows: &[&'a AccountSummary],
    columns: usize,
    height: f32,
) -> Element<'a, Message> {
    row_snap::view(
        &state.scroll,
        theme,
        rows.iter()
            .map(|summary| account_card(theme, summary))
            .collect(),
        columns,
        height,
        Message::Scrolled,
    )
}

/// The type pill, plus an Archived pill for an inactive account.
fn account_badges(theme: Theme, account: &BankAccount) -> Vec<Element<'static, Message>> {
    let kind = account.account_type;
    let mut pills = vec![badges::status_pill(theme, kind.as_str(), type_tone(kind))];
    if !account.is_active {
        pills.push(badges::status_pill(
            theme,
            ARCHIVED_LABEL,
            BadgeTone::Neutral,
        ));
    }
    pills
}

/// One clickable account card: name, masked number, companies, pills,
/// the net balance, both sides' totals and the Edit/Delete buttons.
fn account_card(theme: Theme, summary: &AccountSummary) -> Element<'_, Message> {
    let account = &summary.account;
    let id = summary.id();
    let tone = type_tone(account.account_type);
    let money = |amount: Decimal| labels::currency(&account.currency_code, amount);
    let net = summary.balances.net();
    let actions = row![
        button_style::icon_tool(theme, icons::EDIT, "Edit", Message::EditRow(id), false),
        button_style::icon_tool(theme, icons::REMOVE, "Delete", Message::Remove(id), false),
    ]
    .spacing(spacing::SPACE_4);
    let heading = row![
        badges::icon_box(theme, icons::LANDMARK, tone),
        column![
            labels::strong(theme, account.bank_name.as_str(), typography::panel_title()),
            labels::muted(
                theme,
                labels::masked(&account.account_number_tail()),
                typography::body()
            ),
        ]
        .spacing(spacing::SPACE_2)
        .width(Length::Fill),
        row(account_badges(theme, account)).spacing(spacing::SPACE_4),
        actions,
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let sides = row![
        labels::figure(
            theme,
            BANK_FIELDS[F_CREDIT_BALANCE],
            money(summary.balances.credit),
            labels::side_tone(Side::Credit),
            typography::label(),
        ),
        labels::figure(
            theme,
            BANK_FIELDS[F_DEBIT_BALANCE],
            money(summary.balances.debit),
            labels::side_tone(Side::Debit),
            typography::label(),
        ),
    ]
    .spacing(spacing::MD);
    button(
        column![
            heading,
            labels::muted(theme, companies_line(account), typography::caption()),
            labels::figure(
                theme,
                BANK_FIELDS[F_NET_BALANCE],
                money(net),
                tone,
                typography::subtitle(),
            ),
            sides,
        ]
        .spacing(spacing::SPACE_12),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(button_style::tile(theme, tone))
    .on_press(Message::OpenDetail(id))
    .into()
}

/// The add/edit panel, with a Back button when it replaces the page.
fn form_panel(
    state: &State,
    theme: Theme,
    class: spacing::LayoutClass,
    width: f32,
) -> Element<'_, Message> {
    let mut side = column![]
        .spacing(spacing::SPACE_20)
        .width(Length::Fixed(width))
        .height(Length::Fill);
    if class == spacing::LayoutClass::Narrow {
        side = side.push(button_style::back(theme, Message::CancelEdit));
    }
    side.push(account_form(state, theme)).into()
}

/// A labelled text input for one of `BANK_FIELDS`.
fn input_field<'a>(
    theme: Theme,
    field: usize,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    let label = BANK_FIELDS[field];
    forms::text_field(theme, label, text_input(label, value).on_input(on_input))
}

fn account_form(state: &State, theme: Theme) -> Element<'_, Message> {
    let form = &state.form;
    let fields = vec![
        input_field(
            theme,
            F_BANK_NAME,
            &form.bank_name,
            Message::BankNameChanged,
        ),
        input_field(
            theme,
            F_ACCOUNT_NUMBER,
            &form.account_number,
            Message::AccountNumberChanged,
        ),
        input_field(
            theme,
            F_PARENT_COMPANY,
            &form.parent_company,
            Message::ParentCompanyChanged,
        ),
        input_field(
            theme,
            F_SUB_COMPANY,
            &form.sub_company,
            Message::SubCompanyChanged,
        ),
        forms::field_row(
            theme,
            BANK_FIELDS[F_TYPE],
            dropdown::form_select(
                theme,
                &BANK_ACCOUNT_TYPES[..],
                Some(form.account_type),
                Message::TypeChanged,
            )
            .into(),
        ),
        input_field(
            theme,
            F_OPENING_BALANCE,
            &form.opening_balance,
            Message::OpeningBalanceChanged,
        ),
        forms::field_row(
            theme,
            BANK_FIELDS[F_LEDGER_ACCOUNT],
            dropdown::form_select(
                theme,
                state.gl_accounts.as_slice(),
                form.gl_account.clone(),
                Message::GlAccountSelected,
            )
            .placeholder(BANK_FIELDS[F_LEDGER_ACCOUNT])
            .into(),
        ),
    ];
    let title = if state.editing_id.is_some() {
        "Edit Account"
    } else {
        ADD_LABEL
    };
    let mut body = column![
        labels::strong(theme, title, typography::panel_title()),
        scrollable(container(forms::field_grid(fields, 1)).padding(scrollbar::gutter()))
            .direction(scrollbar::vertical())
            .height(Length::Fill),
    ]
    .spacing(spacing::MD);
    if let Some(error) = &state.form_error {
        body = body.push(text(error.clone()).color(theme.negative_front()));
    }
    body = body.push(
        row![
            button_style::bold("Save", [spacing::SM, spacing::MD])
                .style(button_style::mode(theme, true))
                .on_press(Message::Create),
            button_style::bold("Cancel", [spacing::SM, spacing::MD])
                .style(button_style::outlined(theme, false))
                .on_press(Message::CancelEdit),
        ]
        .spacing(spacing::SM),
    );
    panels::side(theme, body)
}

// -- detail page -----------------------------------------------------------

/// Back/Edit/Delete, the summary figures and the transaction list.
fn detail_page<'a>(
    state: &'a State,
    theme: Theme,
    detail: &'a Detail,
    summary: &'a AccountSummary,
    width: f32,
) -> Element<'a, Message> {
    let account = &summary.account;
    let id = summary.id();
    let subtitle = format!(
        "{}  ·  {}",
        labels::masked(&account.account_number_tail()),
        account.account_number
    );
    let actions = vec![
        button_style::icon_label(icons::EDIT, "Edit", [spacing::SPACE_6, spacing::SPACE_14])
            .style(button_style::mode(theme, true))
            .on_press(Message::EditRow(id))
            .into(),
        button_style::icon_tool(theme, icons::REMOVE, "Delete", Message::Remove(id), true),
    ];
    let mut page = column![page_header::detail(
        theme,
        account.bank_name.as_str(),
        subtitle,
        account_badges(theme, account),
        Message::CloseDetail,
        actions,
    )]
    .spacing(spacing::MD)
    .height(Length::Fill);
    if let Some(notices) = notices(state, theme) {
        page = page.push(notices);
    }
    page.push(detail_summary(theme, detail, summary, width))
        .push(transactions_card(
            theme,
            detail,
            &account.currency_code,
            width,
        ))
        .into()
}

/// Every balance figure plus the companies, all derived from the ledger.
fn detail_summary<'a>(
    theme: Theme,
    detail: &Detail,
    summary: &'a AccountSummary,
    width: f32,
) -> Element<'a, Message> {
    let account = &summary.account;
    let balances = detail.ledger.balances();
    let money = |amount: Decimal| labels::currency(&account.currency_code, amount);
    let plain = |field: usize, value: String| {
        labels::figure(
            theme,
            BANK_FIELDS[field],
            value,
            BadgeTone::Neutral,
            typography::emphasis(),
        )
    };
    let sided = |field: usize, value: String, side: Side| {
        labels::figure(
            theme,
            BANK_FIELDS[field],
            value,
            labels::side_tone(side),
            typography::emphasis(),
        )
    };
    let net = balances.net();
    let figures = vec![
        labels::figure(
            theme,
            BANK_FIELDS[F_NET_BALANCE],
            money(net),
            type_tone(account.account_type),
            typography::subtitle(),
        ),
        sided(F_CREDIT_BALANCE, money(balances.credit), Side::Credit),
        sided(F_DEBIT_BALANCE, money(balances.debit), Side::Debit),
        plain(F_OPENING_BALANCE, money(balances.opening)),
        plain(
            F_LAST_TRANSACTION,
            detail
                .ledger
                .last_transaction_date()
                .map(day)
                .unwrap_or_else(|| EMPTY_DASH.into()),
        ),
        plain(F_PARENT_COMPANY, dash(&account.parent_company)),
        plain(F_SUB_COMPANY, dash(&account.sub_company)),
    ];
    let columns = spacing::grid_columns(spacing::layout_class(width)) + 1;
    panels::padded(theme, forms::field_grid(figures, columns))
}

/// The ledger entries that pass the search, in the chosen order, with
/// their index into the ledger.
fn visible_entries(detail: &Detail) -> Vec<(usize, &BankLedgerEntry)> {
    let query = detail.search.trim().to_lowercase();
    let mut entries: Vec<(usize, &BankLedgerEntry)> = detail
        .ledger
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            query.is_empty()
                || [&entry.reference, &entry.description, &entry.kind]
                    .iter()
                    .any(|value| value.to_lowercase().contains(&query))
        })
        .collect();
    if !detail.oldest_first {
        entries.reverse();
    }
    entries
}

/// Heading with search and order toggle, the rows in their own scroll
/// area, and the pager.
fn transactions_card<'a>(
    theme: Theme,
    detail: &'a Detail,
    currency: &'a str,
    width: f32,
) -> Element<'a, Message> {
    let class = spacing::layout_class(width);
    let (order_glyph, order_tip) = if detail.oldest_first {
        (icons::SORT_ASCENDING, "Oldest first")
    } else {
        (icons::SORT_DESCENDING, "Newest first")
    };
    let heading = row![
        labels::strong(theme, "Transactions", typography::panel_title()).wrapping(Wrapping::None),
        labels::muted(
            theme,
            format!("({})", detail.ledger.entries.len()),
            typography::label()
        ),
    ]
    .spacing(spacing::SPACE_8)
    .align_y(Alignment::Center);
    let tools = button_style::icon_tool(theme, order_glyph, order_tip, Message::ToggleOrder, true);
    let filters = search::input(
        theme,
        "Search reference or description",
        &detail.search,
        Message::DetailSearchChanged,
    );
    let top: Element<'a, Message> = if class == spacing::LayoutClass::Narrow {
        column![heading, row![filters, tools].spacing(spacing::SPACE_12)]
            .spacing(spacing::SPACE_12)
            .into()
    } else {
        row![
            container(heading).width(Length::FillPortion(1)),
            container(filters).width(Length::FillPortion(1)),
            tools
        ]
        .spacing(spacing::SPACE_12)
        .align_y(Alignment::Center)
        .into()
    };
    let mut card = column![container(top).padding(spacing::MD)];

    let entries = paginate(visible_entries(detail), detail.page, detail.page_size);
    let body: Element<'a, Message> = if let Some(error) = &detail.error {
        container(text(error.clone()).color(theme.negative_front()))
            .padding(spacing::MD)
            .into()
    } else if detail.ledger.entries.is_empty() {
        panels::empty_state(
            theme,
            icons::FILE,
            "No transactions yet",
            Some("Posted receipts, payments and journal entries on this account's ledger account appear here."),
            None,
        )
    } else if entries.rows.is_empty() {
        panels::empty_state(
            theme,
            icons::SEARCH,
            "No transactions match this search",
            None,
            None,
        )
    } else {
        let mut rows = column![];
        for (position, (index, entry)) in entries.rows.iter().copied().enumerate() {
            if position > 0 {
                rows = rows.push(table_style::divider_line(theme));
            }
            rows = rows.push(transaction_row(
                theme,
                entry,
                index,
                currency,
                detail.ledger.account_type,
                detail.expanded == Some(index),
            ));
        }
        scrollable(container(rows).padding(scrollbar::gutter()))
            .direction(scrollbar::vertical())
            .height(Length::Fill)
            .into()
    };
    card = card.push(table_style::divider_line(theme)).push(body);
    if entries.pages > 1 {
        card = card.push(table_style::divider_line(theme));
        card = card.push(pagination::footer(pagination::Footer {
            theme,
            class,
            page: entries.page,
            pages: entries.pages,
            shown: entries.rows.len(),
            total: entries.total,
            page_size: detail.page_size,
            page_sizes: &TRANSACTION_PAGE_SIZES,
            on_previous: Message::DetailPrevPage,
            on_page: Message::DetailGoToPage,
            on_page_size: Message::DetailPageSizeChanged,
        }));
    }
    container(card.height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::SPACE_1)
        .style(table_style::card(theme, states::RADIUS_LG))
        .into()
}

/// `28 Sep 2026, 09:52 AM` when the posting time is known.
fn when(entry: &BankLedgerEntry) -> String {
    match entry.posted_at {
        Some(posted) if posted.date() == entry.date => {
            format!("{}, {}", day(entry.date), posted.format("%I:%M %p"))
        }
        _ => day(entry.date),
    }
}

/// One transaction: icon, title, reference and time, the signed amount
/// over the running balance, and Info, which shows the transaction's
/// lines under it.
fn transaction_row<'a>(
    theme: Theme,
    entry: &'a BankLedgerEntry,
    index: usize,
    currency: &'a str,
    kind: BankAccountType,
    expanded: bool,
) -> Element<'a, Message> {
    let movement = kind.net_movement(entry.debit, entry.credit);
    let tone = labels::side_tone(entry_side(entry));
    let glyph = if movement.is_sign_negative() {
        icons::OUTFLOW
    } else {
        icons::INFLOW
    };
    let sign = if movement.is_sign_negative() {
        "-"
    } else {
        "+"
    };
    let title = if entry.description.trim().is_empty() {
        entry.kind.as_str()
    } else {
        entry.description.as_str()
    };
    let main = row![
        badges::icon_box(theme, glyph, tone),
        column![
            labels::strong(theme, title, typography::label()),
            labels::muted(
                theme,
                format!("{}  ·  {}  ·  {}", entry.reference, entry.kind, when(entry)),
                typography::caption()
            ),
        ]
        .spacing(spacing::SPACE_2)
        .width(Length::Fill),
        column![
            labels::amount(
                theme,
                format!("{sign} {}", labels::currency(currency, movement.abs())),
                tone,
                typography::emphasis()
            )
            .wrapping(Wrapping::None),
            labels::muted(
                theme,
                format!(
                    "{} {}",
                    BANK_FIELDS[F_RUNNING_BALANCE],
                    labels::money(entry.running_balance)
                ),
                typography::caption()
            )
            .wrapping(Wrapping::None),
        ]
        .spacing(spacing::SPACE_2)
        .align_x(Alignment::End),
        button_style::icon_label(icons::INFO, "Info", [spacing::SPACE_4, spacing::SPACE_8])
            .style(button_style::outlined(theme, expanded))
            .on_press(Message::ToggleInfo(index)),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let mut block = column![main].spacing(spacing::SPACE_8);
    if expanded {
        let mut lines = column![].spacing(spacing::SPACE_4);
        for line in &entry.lines {
            let (side, amount) = if line.debit.is_zero() {
                (BankAccountType::Credit, line.credit)
            } else {
                (BankAccountType::Debit, line.debit)
            };
            lines = lines.push(
                row![
                    labels::muted(
                        theme,
                        format!("{} - {}", line.account_code, line.account_name),
                        typography::caption()
                    )
                    .width(Length::Fill),
                    labels::muted(
                        theme,
                        format!("{side}  {}", labels::currency(currency, amount)),
                        typography::caption()
                    )
                    .wrapping(Wrapping::None),
                ]
                .spacing(spacing::SPACE_12),
            );
        }
        block = block.push(panels::padded(theme, lines));
    }
    container(block).padding(spacing::SPACE_12).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::banking::bank_ledger::BankLedgerLine;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::banking_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::transactions::transaction::Transaction;
    use crate::backend::transactions::transaction_lines::TransactionLine;
    use crate::backend::transactions::transaction_types::TransactionType;
    use crate::database::database::Database;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    const WIDTHS: [f32; 4] = [640.0, 900.0, 1366.0, 1920.0];
    const THEMES: [Theme; 2] = [Theme::Light, Theme::Dark];

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_a_gl_account() -> (Ctx, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        db.run_migrations(banking_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "PKR"))
            .unwrap()
            .id
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "January 2026",
                date(2026, 1, 1),
                date(2026, 1, 31),
            ))
            .unwrap();
        let cash = accounting_repo
            .create_account(&Account::new(
                company_id,
                "1000",
                "Cash",
                AccountType::Asset,
            ))
            .unwrap();

        let ctx = Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        };
        let choice = Choice {
            id: cash.id.unwrap(),
            label: "1000 - Cash".into(),
        };
        (ctx, choice)
    }

    fn ctx_with_temp_export_dir() -> (Ctx, Choice) {
        let (ctx, choice) = ctx_with_a_gl_account();
        (
            Ctx {
                export_dir: std::env::temp_dir().join(format!(
                    "memonbooks_bank_accounts_view_test_{:?}",
                    std::thread::current().id()
                )),
                ..ctx
            },
            choice,
        )
    }

    /// Adds an account through the panel, the way a user would.
    fn add(state: &mut State, ctx: &Ctx, gl: &Choice, name: &str, number: &str) -> i64 {
        update(state, Message::OpenForm, ctx);
        update(state, Message::GlAccountSelected(gl.clone()), ctx);
        update(state, Message::BankNameChanged(name.into()), ctx);
        update(state, Message::AccountNumberChanged(number.into()), ctx);
        update(state, Message::Create, ctx);
        assert!(state.form_error.is_none(), "{:?}", state.form_error);
        state
            .accounts
            .iter()
            .find(|s| s.account.bank_name == name)
            .unwrap()
            .id()
    }

    fn post(ctx: &Ctx, gl_account_id: i64, day: u32, amount: Decimal, into_bank: bool) {
        let accounting = AccountingRepository::new(ctx.db.clone());
        let other = accounting
            .create_account(&Account::new(
                ctx.company_id,
                format!("6{day:03}"),
                "Offset",
                AccountType::Expense,
            ))
            .unwrap()
            .id
            .unwrap();
        let (bank, offset) = if into_bank {
            (
                TransactionLine::debit(gl_account_id, amount),
                TransactionLine::credit(other, amount),
            )
        } else {
            (
                TransactionLine::credit(gl_account_id, amount),
                TransactionLine::debit(other, amount),
            )
        };
        accounting
            .post_transaction(
                &Transaction::new(
                    ctx.company_id,
                    TransactionType::ManualJournal,
                    date(2026, 1, day),
                )
                .with_line(bank)
                .with_line(offset),
            )
            .unwrap();
    }

    fn sample_account(id: i64, name: &str, kind: BankAccountType, active: bool) -> BankAccount {
        BankAccount {
            id: Some(id),
            parent_company: "Memon Group".into(),
            sub_company: if id % 2 == 0 {
                "Memon Textiles".into()
            } else {
                String::new()
            },
            account_type: kind,
            is_active: active,
            opening_balance: dec!(1000),
            ..BankAccount::new(1, 10, name, format!("PK36HABB{id:08}"), "PKR")
        }
    }

    fn entry(id: i64, day: u32, debit: Decimal, credit: Decimal) -> BankLedgerEntry {
        BankLedgerEntry {
            transaction_id: id,
            date: date(2026, 1, day),
            posted_at: date(2026, 1, day).and_hms_opt(9, 52, 0),
            kind: "Vendor Payment".into(),
            reference: format!("PAY-{id}"),
            description: if id % 2 == 0 {
                String::new()
            } else {
                format!("Payment {id}")
            },
            debit,
            credit,
            running_balance: Decimal::ZERO,
            lines: vec![BankLedgerLine {
                account_code: "1010".into(),
                account_name: "Bank".into(),
                debit,
                credit,
            }],
        }
    }

    fn ledger(count: usize) -> BankLedger {
        BankLedger::new(
            BankAccountType::Debit,
            dec!(1000),
            (0..count)
                .map(|i| {
                    let amount = Decimal::from(10 * (i as i64 + 1));
                    if i % 2 == 0 {
                        entry(i as i64 + 1, (i % 28) as u32 + 1, amount, dec!(0))
                    } else {
                        entry(i as i64 + 1, (i % 28) as u32 + 1, dec!(0), amount)
                    }
                })
                .collect(),
        )
    }

    fn state_with(count: usize) -> State {
        let kinds = [BankAccountType::Debit, BankAccountType::Credit];
        State {
            accounts: (0..count)
                .map(|i| {
                    let name = if i == 0 {
                        "Standard Chartered Bank (Pakistan) Limited, Clifton Corporate Branch"
                            .to_string()
                    } else {
                        format!("Bank {i}")
                    };
                    let account = sample_account(i as i64 + 1, &name, kinds[i % 2], i % 5 != 4);
                    AccountSummary::new(account, &ledger(i % 3))
                })
                .collect(),
            ..State::default()
        }
    }

    /// The page (header and width-dependent content) at every test width
    /// and theme.
    fn build_everywhere(state: &State) {
        for theme in THEMES {
            let _ = view(state, theme);
            let switcher: Element<'_, Message> = text("switcher").into();
            let _ = page_header_view_with_switcher(state, theme, switcher, |m| m);
            for width in WIDTHS {
                let _ = content(state, theme, width);
            }
        }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::OpenForm, &ctx);
        update(
            &mut state,
            Message::GlAccountSelected(gl_account.clone()),
            &ctx,
        );
        update(
            &mut state,
            Message::BankNameChanged("First National".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::AccountNumberChanged("12345".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::ParentCompanyChanged("Memon Group".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::SubCompanyChanged("Memon Textiles".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::TypeChanged(BankAccountType::Credit),
            &ctx,
        );
        update(
            &mut state,
            Message::OpeningBalanceChanged("1,500".into()),
            &ctx,
        );

        assert_eq!(state.form.gl_account, Some(gl_account));
        assert_eq!(state.form.bank_name, "First National");
        assert_eq!(state.form.account_number, "12345");
        assert_eq!(state.form.parent_company, "Memon Group");
        assert_eq!(state.form.sub_company, "Memon Textiles");
        assert_eq!(state.form.account_type, BankAccountType::Credit);
        assert_eq!(state.form.opening_balance, "1,500");
    }

    #[test]
    fn creating_without_a_gl_account_sets_an_error() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();
        update(&mut state, Message::OpenForm, &ctx);
        update(
            &mut state,
            Message::BankNameChanged("First National".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::AccountNumberChanged("12345".into()),
            &ctx,
        );

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.form_error.as_deref(), Some("choose a GL account"));
        assert!(state.accounts.is_empty());
    }

    #[test]
    fn bank_name_and_account_number_are_required_and_the_opening_balance_numeric() {
        let (ctx, gl) = ctx_with_a_gl_account();
        let mut state = State::default();
        update(&mut state, Message::OpenForm, &ctx);
        update(&mut state, Message::GlAccountSelected(gl), &ctx);

        update(&mut state, Message::Create, &ctx);
        assert_eq!(
            state.form_error,
            Some(format!("{} is required.", BANK_FIELDS[F_BANK_NAME]))
        );

        update(
            &mut state,
            Message::BankNameChanged("First National".into()),
            &ctx,
        );
        update(&mut state, Message::Create, &ctx);
        assert_eq!(
            state.form_error,
            Some(format!("{} is required.", BANK_FIELDS[F_ACCOUNT_NUMBER]))
        );

        update(
            &mut state,
            Message::AccountNumberChanged("12345".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::OpeningBalanceChanged("12x".into()),
            &ctx,
        );
        update(&mut state, Message::Create, &ctx);
        assert_eq!(
            state.form_error,
            Some(format!(
                "{} must be a number.",
                BANK_FIELDS[F_OPENING_BALANCE]
            ))
        );
        assert!(state.accounts.is_empty());
        assert!(state.form_open);
    }

    #[test]
    fn a_duplicate_account_number_is_reported_in_the_panel() {
        let (ctx, gl) = ctx_with_a_gl_account();
        let mut state = State::default();
        add(&mut state, &ctx, &gl, "First National", "12345");

        update(&mut state, Message::OpenForm, &ctx);
        update(&mut state, Message::GlAccountSelected(gl), &ctx);
        update(
            &mut state,
            Message::BankNameChanged("Other Bank".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::AccountNumberChanged("12-345".into()),
            &ctx,
        );
        update(&mut state, Message::Create, &ctx);

        assert!(state
            .form_error
            .as_deref()
            .unwrap_or_default()
            .contains("already used"));
        assert_eq!(state.accounts.len(), 1);
    }

    #[test]
    fn creating_a_valid_bank_account_adds_it_with_a_zero_balance_and_closes_the_form() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        assert_eq!(state.currency, "PKR");

        add(&mut state, &ctx, &gl_account, "First National", "12345");

        assert!(state.error.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].account.bank_name, "First National");
        assert_eq!(state.accounts[0].account.currency_code, "PKR");
        assert_eq!(state.accounts[0].balances.net(), Decimal::ZERO);
        assert!(!state.form_open);
        assert!(state.form.bank_name.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            page: 3,
            ..State::default()
        };

        update(&mut state, Message::SearchChanged("first".into()), &ctx);

        assert_eq!(state.search, "first");
        assert_eq!(state.page, 0);
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_bank_account() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl_account, "First National", "12345");

        update(&mut state, Message::EditRow(id), &ctx);

        assert!(state.form_open);
        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.form.bank_name, "First National");
        assert_eq!(state.form.account_number, "12345");
        assert_eq!(state.form.gl_account, Some(gl_account));
        assert_eq!(state.form.opening_balance, "0");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_bank_account_in_place_and_clears_editing_state() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl_account, "First National", "12345");

        update(&mut state, Message::EditRow(id), &ctx);
        update(
            &mut state,
            Message::BankNameChanged("Second National".into()),
            &ctx,
        );
        update(
            &mut state,
            Message::TypeChanged(BankAccountType::Credit),
            &ctx,
        );
        update(
            &mut state,
            Message::OpeningBalanceChanged("250.75".into()),
            &ctx,
        );
        update(&mut state, Message::Create, &ctx);

        assert!(state.form_error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].account.bank_name, "Second National");
        assert_eq!(state.accounts[0].balances.net(), dec!(250.75));

        let service = BankingService::new(ctx.db.clone());
        let reloaded = service.list_bank_accounts(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].bank_name, "Second National");
        assert_eq!(reloaded[0].account_type, BankAccountType::Credit);
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_bank_account() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl_account, "First National", "12345");

        update(&mut state, Message::EditRow(id), &ctx);
        update(
            &mut state,
            Message::BankNameChanged("Should Not Save".into()),
            &ctx,
        );
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(!state.form_open);
        assert_eq!(state.accounts[0].account.bank_name, "First National");
    }

    #[test]
    fn delete_waits_for_confirmation_and_keep_cancels_it() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl_account, "First National", "12345");

        update(&mut state, Message::Remove(id), &ctx);
        assert_eq!(state.pending_removal, Some(id));
        assert_eq!(state.accounts.len(), 1);
        build_everywhere(&state);

        update(&mut state, Message::CancelRemove, &ctx);
        assert!(state.pending_removal.is_none());
        assert_eq!(state.accounts.len(), 1);
    }

    #[test]
    fn confirming_a_delete_removes_an_account_without_transactions() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl_account, "First National", "12345");
        add(&mut state, &ctx, &gl_account, "Second National", "67890");

        update(&mut state, Message::Remove(id), &ctx);
        update(&mut state, Message::ConfirmRemove, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].account.bank_name, "Second National");
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Deleted First National"));
    }

    /// An account still referenced by a bank statement line is archived,
    /// not deleted, so its history survives.
    #[test]
    fn confirming_a_delete_archives_an_account_that_has_transactions() {
        use crate::backend::banking::bank_transactions::BankTransaction;
        use crate::backend::repositories::banking_repository::BankingRepository;

        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let referenced_id = add(&mut state, &ctx, &gl_account, "First National", "12345");
        BankingRepository::new(ctx.db.clone())
            .create_bank_transaction(&BankTransaction::new(
                ctx.company_id,
                referenced_id,
                date(2026, 1, 5),
                "Customer payment",
                dec!(500),
            ))
            .unwrap();

        update(&mut state, Message::Remove(referenced_id), &ctx);
        update(&mut state, Message::ConfirmRemove, &ctx);

        assert_eq!(state.accounts.len(), 1);
        assert!(!state.accounts[0].account.is_active);
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("archived"));
    }

    #[test]
    fn bulk_export_csv_writes_the_filtered_bank_accounts_with_the_field_headers() {
        let (ctx, gl_account) = ctx_with_temp_export_dir();
        let mut state = load(&ctx);
        add(&mut state, &ctx, &gl_account, "First National", "12345");
        add(&mut state, &ctx, &gl_account, "Second National", "67890");
        state.search = "first".into();

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path(EXPORT_FILE)).unwrap();
        assert!(contents.contains("First National") && !contents.contains("Second National"));
        assert!(contents.starts_with(BANK_FIELDS[F_BANK_NAME]));
        assert!(contents.contains(BANK_FIELDS[F_NET_BALANCE]));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_filter_exports_every_bank_account() {
        let (ctx, gl_account) = ctx_with_temp_export_dir();
        let mut state = load(&ctx);
        add(&mut state, &ctx, &gl_account, "First National", "12345");
        add(&mut state, &ctx, &gl_account, "First Union", "22222");

        update(&mut state, Message::BulkExportCsv, &ctx);

        let contents = std::fs::read_to_string(ctx.export_path(EXPORT_FILE)).unwrap();
        assert!(contents.contains("First National") && contents.contains("First Union"));
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Exported 2"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_type_and_status_filters() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            search: "first".into(),
            type_filter: Some(BankAccountType::Credit),
            status_filter: StatusFilter::Inactive,
            page: 2,
            ..State::default()
        };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.type_filter, None);
        assert_eq!(state.status_filter, StatusFilter::All);
        assert_eq!(state.page, 0);
    }

    #[test]
    fn status_filter_narrows_to_only_matching_bank_accounts() {
        let active = sample_account(1, "First National", BankAccountType::Debit, true);
        let inactive = sample_account(2, "Old Bank", BankAccountType::Debit, false);

        assert!(matches_filters(&active, "", None, StatusFilter::All));
        assert!(matches_filters(&active, "", None, StatusFilter::Active));
        assert!(!matches_filters(&active, "", None, StatusFilter::Inactive));

        assert!(matches_filters(&inactive, "", None, StatusFilter::All));
        assert!(!matches_filters(&inactive, "", None, StatusFilter::Active));
        assert!(matches_filters(&inactive, "", None, StatusFilter::Inactive));
    }

    #[test]
    fn type_filter_and_search_cover_the_type_number_and_both_companies() {
        let credit = sample_account(2, "First National", BankAccountType::Credit, true);
        assert!(matches_filters(
            &credit,
            "",
            Some(BankAccountType::Credit),
            StatusFilter::All
        ));
        assert!(!matches_filters(
            &credit,
            "",
            Some(BankAccountType::Debit),
            StatusFilter::All
        ));
        for query in ["first", "pk36habb", "memon group", "textiles"] {
            assert!(
                matches_filters(&credit, query, None, StatusFilter::All),
                "{query}"
            );
        }
        assert!(!matches_filters(&credit, "meezan", None, StatusFilter::All));
    }

    #[test]
    fn type_filter_changed_sets_the_filter_and_returns_to_the_first_page() {
        let (ctx, _gl) = ctx_with_a_gl_account();
        let mut state = State {
            page: 4,
            ..State::default()
        };

        update(
            &mut state,
            Message::TypeFilterChanged(Some(BankAccountType::Debit)),
            &ctx,
        );

        assert_eq!(state.type_filter, Some(BankAccountType::Debit));
        assert_eq!(state.page, 0);
    }

    #[test]
    fn card_balances_come_from_the_posted_transactions_and_the_opening_balance() {
        let (ctx, gl) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl, "First National", "12345");
        update(&mut state, Message::EditRow(id), &ctx);
        update(
            &mut state,
            Message::OpeningBalanceChanged("1000".into()),
            &ctx,
        );
        update(&mut state, Message::Create, &ctx);
        post(&ctx, gl.id, 5, dec!(300), true);
        post(&ctx, gl.id, 7, dec!(120), false);

        let state = load(&ctx);
        let balances = state.accounts[0].balances;
        assert_eq!(balances.debit, dec!(300));
        assert_eq!(balances.credit, dec!(120));
        assert_eq!(balances.net(), dec!(1180));
        assert_eq!(state.accounts[0].last_transaction, Some(date(2026, 1, 7)));
    }

    #[test]
    fn opening_a_card_loads_its_ledger_and_back_returns_to_the_list() {
        let (ctx, gl) = ctx_with_a_gl_account();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, &gl, "First National", "12345");
        post(&ctx, gl.id, 5, dec!(300), true);
        post(&ctx, gl.id, 7, dec!(120), false);

        update(&mut state, Message::OpenDetail(id), &ctx);
        let detail = state.detail.as_ref().unwrap();
        assert_eq!(detail.ledger.entries.len(), 2);
        assert!(detail.error.is_none());
        // Newest first by default, oldest first after the toggle.
        assert_eq!(visible_entries(detail)[0].1.date, date(2026, 1, 7));
        update(&mut state, Message::ToggleOrder, &ctx);
        assert_eq!(
            visible_entries(state.detail.as_ref().unwrap())[0].1.date,
            date(2026, 1, 5)
        );
        update(&mut state, Message::ToggleInfo(1), &ctx);
        assert_eq!(state.detail.as_ref().unwrap().expanded, Some(1));
        build_everywhere(&state);
        update(&mut state, Message::ToggleInfo(1), &ctx);
        assert_eq!(state.detail.as_ref().unwrap().expanded, None);

        update(&mut state, Message::CloseDetail, &ctx);
        assert!(state.detail.is_none());
    }

    #[test]
    fn the_detail_search_matches_reference_and_description() {
        let detail = Detail {
            ledger: ledger(6),
            search: "pay-3".into(),
            ..Detail::new(1)
        };
        let found: Vec<i64> = visible_entries(&detail)
            .iter()
            .map(|(_, e)| e.transaction_id)
            .collect();
        assert_eq!(found, vec![3]);
        let by_description = Detail {
            search: "payment 5".into(),
            ..detail
        };
        assert_eq!(visible_entries(&by_description).len(), 1);
    }

    #[test]
    fn paging_clamps_a_stale_page_index() {
        let paged = paginate((0..30).collect::<Vec<_>>(), 9, 12);
        assert_eq!(paged.pages, 3);
        assert_eq!(paged.page, 2);
        assert_eq!(paged.rows, (24..30).collect::<Vec<_>>());
        let empty = paginate(Vec::<i32>::new(), 2, 12);
        assert_eq!((empty.pages, empty.page, empty.total), (1, 0, 0));
    }

    #[test]
    fn the_list_builds_at_every_width_and_theme_with_few_and_many_cards() {
        for count in [1, 5, 40] {
            build_everywhere(&state_with(count));
        }
        let mut filtered_out = state_with(5);
        filtered_out.search = "no such bank".into();
        build_everywhere(&filtered_out);
    }

    #[test]
    fn the_card_grid_builds_in_every_theme_and_width_with_any_number_of_cards() {
        for theme in THEMES {
            for width in WIDTHS {
                for count in [0, 1, 6, 7, 40] {
                    let mut state = state_with(count);
                    state.scroll.scroll(row_snap::Scroll::Lines(-1.0));
                    let rows: Vec<&AccountSummary> = state.accounts.iter().collect();
                    let columns = spacing::grid_columns(spacing::layout_class(width));
                    for height in [0.0, 300.0, 700.0, 1200.0] {
                        let _ = card_grid(&state, theme, &rows, columns, height);
                    }
                }
            }
        }
    }

    #[test]
    fn twelve_cards_show_two_full_rows_when_the_height_allows() {
        let state = state_with(12);
        let rows: Vec<&AccountSummary> = state.accounts.iter().collect();
        let height = 2.0 * spacing::card_height() + spacing::ROW_GAP;
        for theme in THEMES {
            for width in WIDTHS {
                let columns = spacing::grid_columns(spacing::layout_class(width));
                let visible =
                    row_snap::visible_rows(height, spacing::card_height(), spacing::ROW_GAP);
                let shown =
                    row_snap::visible_cards(state.scroll.first_row(), visible, columns, rows.len());
                assert_eq!(shown.len(), 2 * columns, "{width}");
                let _ = card_grid(&state, theme, &rows, columns, height);
            }
        }
    }

    #[test]
    fn credit_is_red_and_debit_green_by_side_and_net_follows_the_account_type() {
        assert_eq!(labels::side_tone(Side::Credit), BadgeTone::Negative);
        assert_eq!(labels::side_tone(Side::Debit), BadgeTone::Positive);
        assert_eq!(type_tone(BankAccountType::Debit), BadgeTone::Positive);
        assert_eq!(type_tone(BankAccountType::Credit), BadgeTone::Negative);
        // A credit entry is red and a debit entry green on either account type.
        assert_eq!(entry_side(&entry(1, 1, dec!(0), dec!(50))), Side::Credit);
        assert_eq!(entry_side(&entry(2, 1, dec!(50), dec!(0))), Side::Debit);
    }

    #[test]
    fn the_empty_list_builds_at_every_width_and_theme() {
        let mut state = State::default();
        build_everywhere(&state);
        state.message = Some("Deleted First National.".into());
        state.error = Some("database is locked".into());
        build_everywhere(&state);
    }

    #[test]
    fn the_add_form_builds_at_every_width_and_theme() {
        let mut state = state_with(3);
        state.form_open = true;
        build_everywhere(&state);
        state.form_error = Some("choose a GL account".into());
        build_everywhere(&state);
    }

    #[test]
    fn the_edit_form_builds_at_every_width_and_theme() {
        let mut state = state_with(3);
        state.gl_accounts = vec![Choice {
            id: 10,
            label: "1010 - Bank".into(),
        }];
        let summary = state.accounts[1].clone();
        let id = summary.id();
        state.form_open = true;
        state.editing_id = Some(id);
        state.form.bank_name = summary.account.bank_name.clone();
        state.form.gl_account = state.gl_accounts.first().cloned();
        build_everywhere(&state);
        // The panel also opens over the detail page.
        state.detail = Some(Detail {
            ledger: ledger(3),
            ..Detail::new(id)
        });
        build_everywhere(&state);
    }

    #[test]
    fn the_detail_builds_with_zero_one_and_many_transactions() {
        for count in [0, 1, 60] {
            let mut state = state_with(3);
            let mut detail = Detail {
                ledger: ledger(count),
                ..Detail::new(2)
            };
            detail.expanded = count.checked_sub(1);
            state.detail = Some(detail);
            build_everywhere(&state);
        }
        let mut state = state_with(3);
        state.detail = Some(Detail {
            error: Some("bank account 2 not found".into()),
            ..Detail::new(2)
        });
        state.pending_removal = Some(2);
        build_everywhere(&state);
    }

    #[test]
    fn the_cards_follow_the_grid_breakpoints() {
        assert_eq!(spacing::grid_columns(spacing::layout_class(1920.0)), 3);
        assert_eq!(spacing::grid_columns(spacing::layout_class(1366.0)), 3);
        assert_eq!(spacing::grid_columns(spacing::layout_class(900.0)), 2);
        assert_eq!(spacing::grid_columns(spacing::layout_class(640.0)), 1);
        // With the panel open a wide page leaves the grid medium width.
        let split = spacing::panel_split(1366.0, true, spacing::SPACE_20);
        let grid = split.content.unwrap();
        assert_eq!(spacing::grid_columns(spacing::layout_class(grid)), 2);
        assert_eq!(
            spacing::panel_split(640.0, true, spacing::SPACE_20).content,
            None
        );
    }

    #[test]
    fn every_field_index_resolves_to_a_distinct_name() {
        let fields = [
            F_BANK_NAME,
            F_ACCOUNT_NUMBER,
            F_PARENT_COMPANY,
            F_SUB_COMPANY,
            F_TYPE,
            F_OPENING_BALANCE,
            F_LEDGER_ACCOUNT,
            F_CREDIT_BALANCE,
            F_DEBIT_BALANCE,
            F_NET_BALANCE,
            F_LAST_TRANSACTION,
            F_RUNNING_BALANCE,
            F_STATUS,
        ];
        let names: std::collections::HashSet<&str> =
            fields.iter().map(|&f| BANK_FIELDS[f]).collect();
        assert_eq!(names.len(), BANK_FIELDS.len());
        let summary = &state_with(1).accounts[0];
        for field in EXPORT_FIELDS {
            let _ = field_text(&State::default(), summary, field);
        }
        assert_eq!(
            field_text(&State::default(), summary, F_STATUS),
            ACTIVE_LABEL
        );
        assert_eq!(
            field_text(&State::default(), summary, F_NET_BALANCE),
            "1,000.00"
        );
    }

    #[test]
    fn the_companies_line_skips_blank_names() {
        let mut account = sample_account(2, "Bank", BankAccountType::Debit, true);
        assert_eq!(companies_line(&account), "Memon Group  ·  Memon Textiles");
        account.sub_company.clear();
        assert_eq!(companies_line(&account), "Memon Group");
        account.parent_company.clear();
        assert_eq!(companies_line(&account), EMPTY_DASH);
    }
}
