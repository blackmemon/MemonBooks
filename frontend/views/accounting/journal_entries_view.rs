//! List every manual journal entry and reverse a posted one to correct
//! it. Grouped under `accounting/` alongside `journal_entry_view.rs`
//! (the single-entry create-and-post form this list doesn't duplicate)
//! -- posting a *new* entry still happens there; this screen is purely
//! about what's already posted.
//!
//! A posted entry's GL impact is never rewritten in place (Architecture
//! Rules, Section 3): the only correction available is Reverse, which
//! posts a new, dated, linked transaction flipping every line and marks
//! the original Reversed. There is no Draft state reachable here --
//! `AccountingRepository::post_transaction` always writes a transaction
//! as Posted immediately, so unlike Invoices/Bills there's no draft to
//! edit or delete, and no separate Post step.

use chrono::Utc;

use iced::widget::{column, container, scrollable, text};
use iced::{Element, Length};

use crate::backend::services::accounting_service::AccountingService;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_status::TransactionStatus;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    entries: Vec<Transaction>,
    search: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
    /// Fired by a Posted entry row's "Reverse" button, carrying that
    /// transaction's id. No form to fill in first -- unlike a credit or
    /// debit note, a reversal needs no new account choice, since it
    /// just flips the accounts the original entry already specified.
    ReverseForRow(i64),
}

fn status_tone(status: &str) -> BadgeTone {
    match status {
        "posted" => BadgeTone::Positive,
        "reversed" => BadgeTone::Negative,
        _ => BadgeTone::Neutral,
    }
}

pub fn load(ctx: &Ctx) -> State {
    let service = AccountingService::new(ctx.db.clone());
    match service.list_journal_entries(ctx.company_id) {
        Ok(entries) => State { entries, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, "Filters" / "Sort by" are rendered
        // by the shared toolbar component now (Section 6), and the
        // kebab menu has no secondary action -- a journal entry is never
        // edited in place, only reversed, which is this row's primary
        // action (the visible button), not a kebab action.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked | Message::RowAction(_) => {}
        Message::ReverseForRow(transaction_id) => {
            state.message = None;
            state.error = None;
            let today = Utc::now().date_naive();
            let service = AccountingService::new(ctx.db.clone());
            match service.reverse_transaction(transaction_id, today) {
                Ok(reversal) => {
                    if let Some(slot) = state.entries.iter_mut().find(|t| t.id == Some(transaction_id)) {
                        slot.status = TransactionStatus::Reversed;
                        slot.reversed_by_transaction_id = reversal.id;
                    }
                    state.message = Some(format!("Reversed JE-{transaction_id} (reversal #{}).", reversal.id.unwrap_or_default()));
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header = page_header::view(theme, "Journal Entries", "Every manual journal entry posted to this company's books.", "Refresh", Message::Refresh);

    let toolbar = toolbar::view(
        theme,
        &["All Entries"],
        0,
        Message::TabSelected,
        &state.search,
        "Search journal entries",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Entry list", "Reverse a posted entry to correct it -- posted entries are never edited in place.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Transaction> = state
        .entries
        .iter()
        .filter(|entry| {
            query.is_empty()
                || entry.description.as_deref().unwrap_or_default().to_lowercase().contains(&query)
                || entry.id.map(|id| format!("JE-{id}")).unwrap_or_default().to_lowercase().contains(&query)
        })
        .collect();

    let columns = vec![
        ColumnDef::new("Entry", Length::FillPortion(1)).sorted(SortDirection::Descending),
        ColumnDef::new("Date", Length::FillPortion(1)),
        ColumnDef::new("Memo", Length::FillPortion(3)),
        ColumnDef::new("Status", Length::FillPortion(1)),
        ColumnDef::new("Amount", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|entry| {
            let row = Row::new(vec![
                Cell::text(entry.id.map(|id| format!("JE-{id}")).unwrap_or_default()),
                Cell::date(entry.transaction_date.to_string()),
                Cell::text(entry.description.clone().unwrap_or_else(|| "--".to_string())),
                Cell::status(entry.status.as_str(), status_tone(entry.status.as_str())),
                Cell::text(entry.total_debits().to_string()),
            ]);
            if entry.status == TransactionStatus::Posted { row.with_primary_action("Reverse") } else { row }
        })
        .collect();

    let entry_ids: Vec<i64> = filtered.iter().map(|entry| entry.id.unwrap_or_default()).collect();
    let table = data_table::view(
        theme,
        columns,
        rows,
        None,
        |_index, _checked| Message::RowAction(0),
        |_checked| Message::RowAction(0),
        Message::RowAction,
        move |index| Message::ReverseForRow(entry_ids[index]),
    );

    let mut content = column![header, toolbar, heading, table].spacing(20).padding(24).width(Length::Fill);

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
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::transactions::transaction_lines::TransactionLine;
    use crate::backend::transactions::transaction_types::TransactionType;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct Fixture {
        ctx: Ctx,
        cash: i64,
        capital: i64,
        revenue: i64,
    }

    fn fixture() -> Fixture {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let capital = accounting_repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
        let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();

        Fixture {
            ctx: Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") },
            cash: cash.id.unwrap(),
            capital: capital.id.unwrap(),
            revenue: revenue.id.unwrap(),
        }
    }

    /// Posts a manual journal entry debiting `cash` and crediting
    /// `capital` for `amount`, with `memo`, and returns its id.
    fn post_journal_entry(f: &Fixture, memo: &str, amount: rust_decimal::Decimal) -> i64 {
        let service = AccountingService::new(f.ctx.db.clone());
        let mut transaction = Transaction::new(f.ctx.company_id, TransactionType::ManualJournal, date(2026, 1, 10));
        transaction.description = Some(memo.to_string());
        transaction.lines.push(TransactionLine::debit(f.cash, amount));
        transaction.lines.push(TransactionLine::credit(f.capital, amount));
        service.post_transaction(&transaction).unwrap().id.unwrap()
    }

    #[test]
    fn load_lists_only_manual_journal_entries_not_other_transaction_types() {
        let f = fixture();
        post_journal_entry(&f, "Owner contribution", dec!(1000));

        let service = AccountingService::new(f.ctx.db.clone());
        let mut sale = Transaction::new(f.ctx.company_id, TransactionType::SalesInvoice, date(2026, 1, 12));
        sale.lines.push(TransactionLine::debit(f.cash, dec!(200)));
        sale.lines.push(TransactionLine::credit(f.revenue, dec!(200)));
        service.post_transaction(&sale).unwrap();

        let state = load(&f.ctx);

        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].description.as_deref(), Some("Owner contribution"));
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let f = fixture();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("owner".into()), &f.ctx);

        assert_eq!(state.search, "owner");
    }

    #[test]
    fn refresh_reloads_the_entry_list_from_the_database() {
        let f = fixture();
        post_journal_entry(&f, "Owner contribution", dec!(1000));
        let mut state = State::default();

        update(&mut state, Message::Refresh, &f.ctx);

        assert_eq!(state.entries.len(), 1);
    }

    #[test]
    fn reversing_a_posted_entry_marks_it_reversed_and_posts_a_balanced_reversal() {
        let f = fixture();
        let entry_id = post_journal_entry(&f, "Owner contribution", dec!(1000));
        let mut state = load(&f.ctx);

        update(&mut state, Message::ReverseForRow(entry_id), &f.ctx);

        assert!(state.error.is_none());
        assert!(state.message.as_deref().unwrap_or_default().starts_with("Reversed"));
        let updated = state.entries.iter().find(|t| t.id == Some(entry_id)).unwrap();
        assert_eq!(updated.status, TransactionStatus::Reversed);
        assert!(updated.reversed_by_transaction_id.is_some());

        // The reversal itself is a real, separate, balanced transaction
        // -- not just a status flip.
        let accounting_repo = AccountingRepository::new(f.ctx.db.clone());
        let reloaded_original = accounting_repo.get_transaction(entry_id).unwrap();
        let reversal = accounting_repo.get_transaction(reloaded_original.reversed_by_transaction_id.unwrap()).unwrap();
        assert!(reversal.is_balanced());
        assert_eq!(reversal.transaction_type, TransactionType::Reversal);

        // The ledger nets back to zero across both entries.
        let trial_balance = accounting_repo.trial_balance_as_of(f.ctx.company_id, date(2030, 1, 1)).unwrap();
        assert!(trial_balance.is_balanced());
        assert_eq!(trial_balance.total_debits(), dec!(0));
    }

    #[test]
    fn reversing_an_already_reversed_entry_is_rejected_and_leaves_it_untouched() {
        let f = fixture();
        let entry_id = post_journal_entry(&f, "Owner contribution", dec!(1000));
        let mut state = load(&f.ctx);
        update(&mut state, Message::ReverseForRow(entry_id), &f.ctx);
        assert!(state.error.is_none());
        let first_reversal_id = state.entries.iter().find(|t| t.id == Some(entry_id)).unwrap().reversed_by_transaction_id;

        update(&mut state, Message::ReverseForRow(entry_id), &f.ctx);

        assert!(state.error.is_some());
        let unchanged = state.entries.iter().find(|t| t.id == Some(entry_id)).unwrap();
        assert_eq!(unchanged.status, TransactionStatus::Reversed);
        assert_eq!(unchanged.reversed_by_transaction_id, first_reversal_id, "the original reversal link must not be disturbed by the rejected second attempt");
    }
}
