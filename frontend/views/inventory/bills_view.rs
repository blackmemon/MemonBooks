//! List bills and create + post a simple single-line bill. Mirrors
//! `sales::invoices_view`.

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::purchases::bills::{Bill, BillLine};
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::backend::repositories::purchase_repository::PurchaseRepository;
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
    bills: Vec<Bill>,
    vendors: Vec<Choice>,
    accounts: Vec<Choice>,
    vendor: Option<Choice>,
    ap_account: Option<Choice>,
    expense_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    VendorSelected(Choice),
    ApAccountSelected(Choice),
    ExpenseAccountSelected(Choice),
    DescriptionChanged(String),
    AmountChanged(String),
    CreateAndPost,
}

pub fn load(ctx: &Ctx) -> State {
    let purchase_repo = PurchaseRepository::new(ctx.db.clone());
    let party_repo = PartyRepository::new(ctx.db.clone());
    let accounting_repo = AccountingRepository::new(ctx.db.clone());

    let vendors = party_repo
        .list_vendors_for_company(ctx.company_id)
        .map(|vendors| vendors.iter().map(|v| Choice { id: v.id.unwrap_or_default(), label: v.name.clone() }).collect())
        .unwrap_or_default();
    let accounts: Vec<Choice> = accounting_repo
        .list_accounts_for_company(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();
    let bills = purchase_repo.list_bills_for_company(ctx.company_id).unwrap_or_default();

    State {
        bills,
        vendors,
        accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::VendorSelected(choice) => state.vendor = Some(choice),
        Message::ApAccountSelected(choice) => state.ap_account = Some(choice),
        Message::ExpenseAccountSelected(choice) => state.expense_account = Some(choice),
        Message::DescriptionChanged(value) => state.description_input = value,
        Message::AmountChanged(value) => state.amount_input = value,
        Message::CreateAndPost => {
            state.message = None;
            state.error = None;

            let (Some(vendor), Some(ap_account), Some(expense_account)) = (&state.vendor, &state.ap_account, &state.expense_account) else {
                state.error = Some("choose a vendor, AP account, and expense account".into());
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
            let mut bill = Bill::new(ctx.company_id, vendor.id, today, due_date);
            bill.lines.push(BillLine::new(expense_account.id, state.description_input.clone(), Decimal::ONE, amount));

            let repo = PurchaseRepository::new(ctx.db.clone());
            let created = match repo.create_bill(&bill) {
                Ok(bill) => bill,
                Err(err) => {
                    state.error = Some(err.to_string());
                    return;
                }
            };
            match repo.post_bill(created.id.unwrap_or_default(), ap_account.id) {
                Ok((posted, _transaction, _payable)) => {
                    state.message = Some(format!("Posted bill #{}.", posted.id.unwrap_or_default()));
                    state.bills.push(posted);
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
        text("Bill").width(Length::Fixed(80.0)),
        text("Date").width(Length::Fixed(100.0)),
        text("Status").width(Length::Fixed(80.0)),
        text("Total").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for bill in &state.bills {
        list = list.push(row![
            text(bill.id.map(|id| format!("BILL-{id}")).unwrap_or_default()).width(Length::Fixed(80.0)),
            text(bill.bill_date.to_string()).width(Length::Fixed(100.0)),
            text(bill.status.as_str()).width(Length::Fixed(80.0)),
            text(bill.total().to_string()).width(Length::Fixed(100.0)),
        ]);
    }

    let form = column![
        row![
            pick_list(state.vendors.as_slice(), state.vendor.clone(), Message::VendorSelected).placeholder("Vendor"),
            pick_list(state.accounts.as_slice(), state.ap_account.clone(), Message::ApAccountSelected).placeholder("AP account"),
            pick_list(state.accounts.as_slice(), state.expense_account.clone(), Message::ExpenseAccountSelected).placeholder("Expense account"),
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

    let mut content = column![text("Bills").size(24), scrollable(list).height(Length::Fill), form]
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
