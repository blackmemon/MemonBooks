//! A minimal point-of-sale screen: pick (or open) a session on a
//! terminal, then ring up a single-line cash sale.

use chrono::Utc;
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

use crate::backend::inventory::products::Product;
use crate::backend::pos::payments::{PaymentMethod, PosPayment};
use crate::backend::pos::sales::{PosSale, PosSaleLine};
use crate::backend::pos::sessions::PosSession;
use crate::backend::pos::terminals::Terminal;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::inventory_repository::InventoryRepository;
use crate::backend::repositories::sales_repository::SalesRepository;
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
    terminals: Vec<Choice>,
    products: Vec<Choice>,
    revenue_accounts: Vec<Choice>,
    cash_accounts: Vec<Choice>,
    terminal: Option<Choice>,
    product: Option<Choice>,
    revenue_account: Option<Choice>,
    cash_account: Option<Choice>,
    open_session_id: Option<i64>,
    price_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TerminalSelected(Choice),
    ProductSelected(Choice),
    RevenueAccountSelected(Choice),
    CashAccountSelected(Choice),
    PriceChanged(String),
    OpenSession,
    RingUpCashSale,
}

pub fn load(ctx: &Ctx) -> State {
    let sales_repo = SalesRepository::new(ctx.db.clone());
    let inventory_repo = InventoryRepository::new(ctx.db.clone());
    let accounting_repo = AccountingRepository::new(ctx.db.clone());

    let terminals = sales_repo
        .list_terminals_for_company(ctx.company_id)
        .map(|terminals: Vec<Terminal>| terminals.iter().map(|t| Choice { id: t.id.unwrap_or_default(), label: t.name.clone() }).collect())
        .unwrap_or_default();
    let products = inventory_repo
        .list_products_for_company(ctx.company_id)
        .map(|products: Vec<Product>| products.iter().map(|p| Choice { id: p.id.unwrap_or_default(), label: p.name.clone() }).collect())
        .unwrap_or_default();
    let accounts: Vec<Choice> = accounting_repo
        .list_accounts_for_company(ctx.company_id)
        .map(|accounts| accounts.iter().map(|a: &crate::backend::accounting::chart_of_accounts::Account| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    State {
        terminals,
        products,
        revenue_accounts: accounts.clone(),
        cash_accounts: accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::TerminalSelected(choice) => {
            state.terminal = Some(choice);
            state.open_session_id = None;
        }
        Message::ProductSelected(choice) => state.product = Some(choice),
        Message::RevenueAccountSelected(choice) => state.revenue_account = Some(choice),
        Message::CashAccountSelected(choice) => state.cash_account = Some(choice),
        Message::PriceChanged(value) => state.price_input = value,
        Message::OpenSession => {
            let Some(terminal) = &state.terminal else {
                state.error = Some("choose a terminal first".into());
                return;
            };
            let repo = SalesRepository::new(ctx.db.clone());
            let session = PosSession::open(terminal.id, ctx.current_user_id.unwrap_or_default(), Decimal::ZERO);
            match repo.open_session(&session) {
                Ok(opened) => {
                    state.open_session_id = opened.id;
                    state.message = Some("Session opened.".into());
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::RingUpCashSale => {
            state.message = None;
            state.error = None;

            let (Some(terminal), Some(product), Some(revenue_account), Some(cash_account), Some(session_id)) =
                (&state.terminal, &state.product, &state.revenue_account, &state.cash_account, state.open_session_id)
            else {
                state.error = Some("open a session and choose a product, revenue account, and cash account".into());
                return;
            };
            let price: Decimal = match state.price_input.parse() {
                Ok(price) => price,
                Err(_) => {
                    state.error = Some("price is not a valid number".into());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let mut sale = PosSale::new(ctx.company_id, terminal.id, session_id, today);
            sale.lines.push(PosSaleLine::new(product.id, revenue_account.id, product.label.clone(), Decimal::ONE, price));
            sale.payments.push(PosPayment::new(PaymentMethod::Cash, price));

            let repo = SalesRepository::new(ctx.db.clone());
            match repo.post_pos_sale(&sale, &[(PaymentMethod::Cash, cash_account.id)]) {
                Ok((posted, _transaction)) => {
                    state.message = Some(format!("Sale #{} rung up.", posted.id.unwrap_or_default()));
                    state.price_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let session_status = match state.open_session_id {
        Some(id) => text(format!("Session #{id} open")).color(colors::WHITE_POSITIVE_FRONT),
        None => text("No session open").color(colors::WHITE_WARNING_FRONT),
    };

    let mut content = column![
        text("Point of Sale").size(24),
        pick_list(state.terminals.as_slice(), state.terminal.clone(), Message::TerminalSelected).placeholder("Terminal"),
        row![session_status, button(text("Open session")).on_press(Message::OpenSession).padding(6)].spacing(12),
        pick_list(state.products.as_slice(), state.product.clone(), Message::ProductSelected).placeholder("Product"),
        pick_list(state.revenue_accounts.as_slice(), state.revenue_account.clone(), Message::RevenueAccountSelected).placeholder("Revenue account"),
        pick_list(state.cash_accounts.as_slice(), state.cash_account.clone(), Message::CashAccountSelected).placeholder("Cash account"),
        row![
            text_input("Price", &state.price_input).on_input(Message::PriceChanged).width(Length::Fixed(120.0)).padding(6),
            button(text("Ring up cash sale")).on_press(Message::RingUpCashSale).padding(6),
        ]
        .spacing(8),
    ]
    .spacing(10)
    .padding(20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
