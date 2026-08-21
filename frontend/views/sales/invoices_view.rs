//! List invoices and create + post a simple single-line invoice.

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::backend::repositories::sales_repository::SalesRepository;
use crate::backend::sales::invoices::{Invoice, InvoiceLine};
use crate::frontend::styles::colors;
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

#[derive(Debug, Clone, Default)]
pub struct State {
    invoices: Vec<Invoice>,
    customers: Vec<Choice>,
    accounts: Vec<Choice>,
    customer: Option<Choice>,
    ar_account: Option<Choice>,
    revenue_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CustomerSelected(Choice),
    ArAccountSelected(Choice),
    RevenueAccountSelected(Choice),
    DescriptionChanged(String),
    AmountChanged(String),
    CreateAndPost,
}

pub fn load(ctx: &Ctx) -> State {
    let sales_repo = SalesRepository::new(ctx.db.clone());
    let party_repo = PartyRepository::new(ctx.db.clone());
    let accounting_repo = AccountingRepository::new(ctx.db.clone());

    let customers = party_repo
        .list_customers_for_company(ctx.company_id)
        .map(|customers| customers.iter().map(|c| Choice { id: c.id.unwrap_or_default(), label: c.name.clone() }).collect())
        .unwrap_or_default();
    let accounts: Vec<Choice> = accounting_repo
        .list_accounts_for_company(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    let invoices = sales_repo.list_invoices_for_company(ctx.company_id).unwrap_or_default();

    State {
        invoices,
        customers,
        accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CustomerSelected(choice) => state.customer = Some(choice),
        Message::ArAccountSelected(choice) => state.ar_account = Some(choice),
        Message::RevenueAccountSelected(choice) => state.revenue_account = Some(choice),
        Message::DescriptionChanged(value) => state.description_input = value,
        Message::AmountChanged(value) => state.amount_input = value,
        Message::CreateAndPost => {
            state.message = None;
            state.error = None;

            let (Some(customer), Some(ar_account), Some(revenue_account)) = (&state.customer, &state.ar_account, &state.revenue_account) else {
                state.error = Some("choose a customer, AR account, and revenue account".into());
                return;
            };
            let amount: Decimal = match state.amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let due_date = today.checked_add_days(Days::new(30)).unwrap_or(today);
            let mut invoice = Invoice::new(ctx.company_id, customer.id, today, due_date);
            invoice.lines.push(InvoiceLine::new(revenue_account.id, state.description_input.clone(), Decimal::ONE, amount));

            let repo = SalesRepository::new(ctx.db.clone());
            let created = match repo.create_invoice(&invoice) {
                Ok(invoice) => invoice,
                Err(err) => {
                    state.error = Some(err.to_string());
                    return;
                }
            };
            match repo.post_invoice(created.id.unwrap_or_default(), ar_account.id) {
                Ok((posted, _transaction, _receivable)) => {
                    state.message = Some(format!("Posted invoice #{}.", posted.id.unwrap_or_default()));
                    state.invoices.push(posted);
                    state.description_input.clear();
                    state.amount_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Invoice").width(Length::Fixed(80.0)),
        text("Date").width(Length::Fixed(100.0)),
        text("Status").width(Length::Fixed(80.0)),
        text("Total").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for invoice in &state.invoices {
        list = list.push(row![
            text(invoice.id.map(|id| format!("INV-{id}")).unwrap_or_default()).width(Length::Fixed(80.0)),
            text(invoice.invoice_date.to_string()).width(Length::Fixed(100.0)),
            text(invoice.status.as_str()).width(Length::Fixed(80.0)),
            text(invoice.total().to_string()).width(Length::Fixed(100.0)),
        ]);
    }

    let form = column![
        row![
            pick_list(state.customers.as_slice(), state.customer.clone(), Message::CustomerSelected).placeholder("Customer"),
            pick_list(state.accounts.as_slice(), state.ar_account.clone(), Message::ArAccountSelected).placeholder("AR account"),
            pick_list(state.accounts.as_slice(), state.revenue_account.clone(), Message::RevenueAccountSelected).placeholder("Revenue account"),
        ]
        .spacing(8),
        row![
            text_input("Description", &state.description_input).on_input(Message::DescriptionChanged).width(Length::Fill).padding(6),
            text_input("Amount", &state.amount_input).on_input(Message::AmountChanged).width(Length::Fixed(120.0)).padding(6),
            button(text("Create & post")).on_press(Message::CreateAndPost).padding(6),
        ]
        .spacing(8),
    ]
    .spacing(8);

    let mut content = column![text("Invoices").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
