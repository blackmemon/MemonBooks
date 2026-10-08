//! The landing screen after login: a snapshot of the company's books.

use chrono::Utc;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::core::errors::AppResult;
use crate::backend::services::document_service::DocumentService;
use crate::backend::services::report_service::ReportService;
use crate::backend::transactions::documents::{num, Document, Kind};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
use crate::frontend::components::{button as buttons, panels};
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub expenses: Vec<Document>,
    pub trips: usize,
    pub pending: usize,
    pub total_assets: String,
    pub total_liabilities: String,
    pub total_equity: String,
    pub open_receivables_count: usize,
    pub open_payables_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    NewExpense,
    CreateReport,
}

pub fn load(ctx: &Ctx) -> State {
    match load_summary(ctx) {
        Ok(state) => state,
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
    }
}

fn load_summary(ctx: &Ctx) -> AppResult<State> {
    let service = ReportService::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    let trial_balance = service.trial_balance_as_of(ctx.company_id, today)?;
    let workflows = DocumentService::new(ctx.db.clone());
    let expenses = workflows.list(ctx.company_id, Kind::Expense)?;
    let trips = workflows.list(ctx.company_id, Kind::Trip)?.len();
    let mut pending = 0;
    for kind in crate::backend::transactions::documents::ALL
        .iter()
        .filter(|k| k.posts())
    {
        pending += workflows
            .list(ctx.company_id, *kind)?
            .iter()
            .filter(|d| d.status == "Pending Approval")
            .count();
    }

    Ok(State {
        expenses,
        trips,
        pending,
        total_assets: trial_balance.total_debits().to_string(),
        total_liabilities: String::new(),
        total_equity: trial_balance.total_credits().to_string(),
        open_receivables_count: service.list_open_receivables(ctx.company_id)?.len(),
        open_payables_count: service.list_open_payables(ctx.company_id)?.len(),
        error: None,
    })
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    if let Message::Refresh = message {
        *state = load(ctx);
    }
}

fn summary_card(
    theme: Theme,
    label: impl Into<String>,
    value: impl Into<String>,
) -> Element<'static, Message> {
    container(
        column![
            text(label.into())
                .size(typography::emphasis())
                .color(theme.primary()),
            text(value.into())
                .size(typography::stat_value())
                .color(theme.text_primary())
        ]
        .spacing(spacing::SPACE_6),
    )
    .padding(spacing::MD)
    .width(Length::FillPortion(1))
    .into()
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    if let Some(error) = &state.error {
        return container(
            text(format!("Could not load dashboard: {error}")).color(theme.negative_front()),
        )
        .padding(spacing::SPACE_20)
        .into();
    }

    let pending = column![
        text("Pending Tasks").size(typography::subtitle()),
        text(format!(
            "Pending Approvals                         {}",
            state.pending
        )),
        text(format!(
            "Trips Registered                             {}",
            state.trips
        )),
        text(format!(
            "Unreported Expenses                      {}",
            state
                .expenses
                .iter()
                .filter(|d| d.get("Report Name").is_empty())
                .count()
        )),
        text(format!(
            "Open Receivables                            {}",
            state.open_receivables_count
        )),
        text(format!(
            "Open Payables                                 {}",
            state.open_payables_count
        ))
    ]
    .spacing(spacing::SPACE_24);
    let mut recent = column![
        text("Recent Expenses").size(typography::subtitle()),
        row![
            text("Subject").width(Length::Fill),
            text("Employee").width(Length::Fill),
            text("Amount").width(Length::Fill)
        ]
    ]
    .spacing(spacing::SPACE_18);
    for d in state.expenses.iter().take(5) {
        recent = recent.push(row![
            text(d.get("Subject")).width(Length::Fill),
            text(d.get("Employee Name")).width(Length::Fill),
            text(format!("{} {}", d.get("Currency"), d.get("Total"))).width(Length::Fill)
        ]);
    }
    if state.expenses.is_empty() {
        recent = recent.push(text("No expenses yet. Add your first expense below."));
    }
    let mut quick = row![].spacing(spacing::SPACE_12);
    for (label, message) in [
        ("+ New expense", Message::NewExpense),
        ("+ Create report", Message::CreateReport),
    ] {
        quick = quick.push(
            button(text(label))
                .on_press(message)
                .padding(spacing::MD)
                .width(Length::Fill)
                .style(buttons::primary(theme)),
        );
    }
    let mut categories = std::collections::BTreeMap::<String, rust_decimal::Decimal>::new();
    let mut dates = std::collections::BTreeMap::<String, rust_decimal::Decimal>::new();
    for d in &state.expenses {
        if let Ok(amount) = num(&d.fields, "Total") {
            *categories
                .entry(format!("{} ({})", d.get("Category"), d.get("Currency")))
                .or_default() += amount;
            *dates
                .entry(format!("{} ({})", d.get("Date"), d.get("Currency")))
                .or_default() += amount;
        }
    }
    let breakdown =
        |title: &str, values: std::collections::BTreeMap<String, rust_decimal::Decimal>| {
            let mut c = column![text(title.to_string()).size(typography::subtitle())]
                .spacing(spacing::SPACE_14);
            if values.is_empty() {
                c = c.push(text("Expense activity will appear here after saving."));
            }
            for (label, value) in values {
                c = c.push(row![
                    text(label).width(Length::Fill),
                    text(value.to_string())
                ]);
            }
            container(c)
                .padding(spacing::SPACE_24)
                .width(Length::Fill)
                .style(panels::flat_card(theme, 10.))
        };
    scrollable(
        column![
            text("Home").size(typography::title()),
            row![
                container(pending)
                    .padding(spacing::SPACE_24)
                    .width(Length::Fill)
                    .style(panels::flat_card(theme, 10.)),
                container(recent)
                    .padding(spacing::SPACE_24)
                    .width(Length::Fill)
                    .style(panels::flat_card(theme, 10.))
            ]
            .spacing(spacing::SPACE_20),
            container(
                column![text("Quick Access").size(typography::subtitle()), quick]
                    .spacing(spacing::SPACE_18)
            )
            .padding(spacing::SPACE_24)
            .style(panels::flat_card(theme, 10.)),
            row![
                breakdown("Spending by Category", categories),
                breakdown("Day-to-Day Expenses", dates)
            ]
            .spacing(spacing::SPACE_20),
            row![
                summary_card(theme, "Ledger Debits", state.total_assets.clone()),
                summary_card(theme, "Ledger Credits", state.total_equity.clone())
            ]
        ]
        .spacing(spacing::SPACE_24)
        .padding(spacing::SPACE_24),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::accounts_payable::payables::Payable;
    use crate::backend::accounting::accounts_receivable::receivables::Receivable;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::parties::customers::Customer;
    use crate::backend::parties::vendors::Vendor;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository::{self, PartyRepository};
    use crate::backend::transactions::transaction::Transaction;
    use crate::backend::transactions::transaction_lines::TransactionLine;
    use crate::backend::transactions::transaction_types::TransactionType;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_ledger_activity() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::repositories::document_repository::MIGRATIONS)
            .unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        let party_repo = PartyRepository::new(db.clone());
        let customer_id = party_repo
            .create_customer(&Customer::new(company_id, "Acme Retail"))
            .unwrap()
            .id
            .unwrap();
        let vendor_id = party_repo
            .create_vendor(&Vendor::new(company_id, "Acme Supplies"))
            .unwrap()
            .id
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                date(2026, 1, 1),
                date(2030, 12, 31),
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
        let ar = accounting_repo
            .create_account(&Account::new(
                company_id,
                "1100",
                "Accounts Receivable",
                AccountType::Asset,
            ))
            .unwrap();
        let ap = accounting_repo
            .create_account(&Account::new(
                company_id,
                "2000",
                "Accounts Payable",
                AccountType::Liability,
            ))
            .unwrap();
        let revenue = accounting_repo
            .create_account(&Account::new(
                company_id,
                "4000",
                "Sales Revenue",
                AccountType::Revenue,
            ))
            .unwrap();

        accounting_repo
            .post_transaction(
                &Transaction::new(
                    company_id,
                    TransactionType::OpeningBalance,
                    date(2026, 1, 1),
                )
                .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(5000)))
                .with_line(TransactionLine::credit(ar.id.unwrap(), dec!(5000))),
            )
            .unwrap();

        accounting_repo
            .post_transaction_with_receivable(
                &Transaction::new(company_id, TransactionType::SalesInvoice, date(2026, 1, 5))
                    .with_line(TransactionLine::debit(ar.id.unwrap(), dec!(1000)))
                    .with_line(TransactionLine::credit(revenue.id.unwrap(), dec!(1000))),
                &Receivable::new(
                    company_id,
                    customer_id,
                    date(2026, 1, 5),
                    date(2026, 2, 4),
                    dec!(1000),
                ),
            )
            .unwrap();

        accounting_repo
            .post_transaction_with_payable(
                &Transaction::new(company_id, TransactionType::PurchaseBill, date(2026, 1, 6))
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(300)))
                    .with_line(TransactionLine::credit(ap.id.unwrap(), dec!(300))),
                &Payable::new(
                    company_id,
                    vendor_id,
                    date(2026, 1, 6),
                    date(2026, 2, 5),
                    dec!(300),
                ),
            )
            .unwrap();

        Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        }
    }

    #[test]
    fn refresh_loads_the_trial_balance_totals_and_open_item_counts() {
        let ctx = ctx_with_ledger_activity();
        let mut state = State::default();

        update(&mut state, Message::Refresh, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.open_receivables_count, 1);
        assert_eq!(state.open_payables_count, 1);
        assert_eq!(state.total_assets, state.total_equity); // trial balance stays balanced
    }
}
