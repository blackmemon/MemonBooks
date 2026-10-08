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
use crate::backend::services::sales_service::SalesService;
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::dropdown;
use crate::frontend::components::inputs;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
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
    let service = SalesService::new(ctx.db.clone());

    let terminals = service
        .list_terminals(ctx.company_id)
        .map(|terminals: Vec<Terminal>| {
            terminals
                .iter()
                .map(|t| Choice {
                    id: t.id.unwrap_or_default(),
                    label: t.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let products = service
        .list_products(ctx.company_id)
        .map(|products: Vec<Product>| {
            products
                .iter()
                .map(|p| Choice {
                    id: p.id.unwrap_or_default(),
                    label: p.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let accounts: Vec<Choice> = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts| {
            accounts
                .iter()
                .map(
                    |a: &crate::backend::accounting::chart_of_accounts::Account| Choice {
                        id: a.id.unwrap_or_default(),
                        label: format!("{} - {}", a.account_code, a.account_name),
                    },
                )
                .collect()
        })
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
            let service = SalesService::new(ctx.db.clone());
            let session = PosSession::open(
                terminal.id,
                ctx.current_user_id.unwrap_or_default(),
                Decimal::ZERO,
            );
            match service.open_session(&session) {
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

            let (
                Some(terminal),
                Some(product),
                Some(revenue_account),
                Some(cash_account),
                Some(session_id),
            ) = (
                &state.terminal,
                &state.product,
                &state.revenue_account,
                &state.cash_account,
                state.open_session_id,
            )
            else {
                state.error = Some(
                    "open a session and choose a product, revenue account, and cash account".into(),
                );
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
            sale.lines.push(PosSaleLine::new(
                product.id,
                revenue_account.id,
                product.label.clone(),
                Decimal::ONE,
                price,
            ));
            sale.payments
                .push(PosPayment::new(PaymentMethod::Cash, price));

            let service = SalesService::new(ctx.db.clone());
            match service.post_pos_sale(&sale, &[(PaymentMethod::Cash, cash_account.id)]) {
                Ok((posted, _transaction)) => {
                    state.message =
                        Some(format!("Sale #{} rung up.", posted.id.unwrap_or_default()));
                    state.price_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let session_status = match state.open_session_id {
        Some(id) => text(format!("Session #{id} open")).color(theme.positive_front()),
        None => text("No session open").color(theme.warning_front()),
    };

    let mut content = column![
        text("Point of Sale").size(typography::title()),
        pick_list(
            state.terminals.as_slice(),
            state.terminal.clone(),
            Message::TerminalSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Terminal"),
        row![
            session_status,
            button(text("Open session"))
                .style(button_style::primary(theme))
                .on_press(Message::OpenSession)
                .padding(spacing::SPACE_6)
        ]
        .spacing(spacing::SPACE_12),
        pick_list(
            state.products.as_slice(),
            state.product.clone(),
            Message::ProductSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Product"),
        pick_list(
            state.revenue_accounts.as_slice(),
            state.revenue_account.clone(),
            Message::RevenueAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Revenue account"),
        pick_list(
            state.cash_accounts.as_slice(),
            state.cash_account.clone(),
            Message::CashAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Cash account"),
        row![
            text_input("Price", &state.price_input)
                .style(inputs::text(theme))
                .on_input(Message::PriceChanged)
                .width(Length::Fixed(120.0))
                .padding(spacing::SPACE_6),
            button(text("Ring up cash sale"))
                .style(button_style::primary(theme))
                .on_press(Message::RingUpCashSale)
                .padding(spacing::SPACE_6),
        ]
        .spacing(spacing::SM),
    ]
    .spacing(spacing::SPACE_10)
    .padding(spacing::SPACE_20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(content).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::core::roles::Role;
    use crate::backend::core::users::User;
    use crate::backend::inventory::warehouses::Warehouse;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::inventory_repository::{self, InventoryRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::sales_repository::{self, SalesRepository};
    use crate::backend::repositories::user_repository::{self, UserRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_terminal_product_and_accounts() -> (Ctx, Choice, Choice, Choice, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        db.run_migrations(inventory_repository::MIGRATIONS).unwrap();
        db.run_migrations(sales_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        let user_repo = UserRepository::new(db.clone());
        let role = user_repo.create_role(&Role::new("Cashier")).unwrap();
        let cashier = user_repo
            .create_user(&User::new(
                "cashier",
                "Cashier One",
                "hash",
                role.id.unwrap(),
            ))
            .unwrap();

        let inventory_repo = InventoryRepository::new(db.clone());
        let warehouse = inventory_repo
            .create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN"))
            .unwrap();
        let product = inventory_repo
            .create_product(&crate::backend::inventory::products::Product::new(
                company_id, "WIDGET-1", "Widget",
            ))
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(
                &crate::backend::accounting::fiscal_periods::FiscalPeriod::new(
                    company_id,
                    "Open Period",
                    chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
                ),
            )
            .unwrap();
        let revenue = accounting_repo
            .create_account(&Account::new(
                company_id,
                "4000",
                "Sales Revenue",
                AccountType::Revenue,
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

        let sales_repo = SalesRepository::new(db.clone());
        let terminal = sales_repo
            .create_terminal(&Terminal::new(
                company_id,
                warehouse.id.unwrap(),
                "Front Counter",
            ))
            .unwrap();

        let ctx = Ctx {
            db,
            company_id,
            current_user_id: cashier.id,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        };
        let terminal_choice = Choice {
            id: terminal.id.unwrap(),
            label: terminal.name.clone(),
        };
        let product_choice = Choice {
            id: product.id.unwrap(),
            label: product.name.clone(),
        };
        let revenue_choice = Choice {
            id: revenue.id.unwrap(),
            label: "4000 - Sales Revenue".into(),
        };
        let cash_choice = Choice {
            id: cash.id.unwrap(),
            label: "1000 - Cash".into(),
        };
        (
            ctx,
            terminal_choice,
            product_choice,
            revenue_choice,
            cash_choice,
        )
    }

    #[test]
    fn selecting_a_different_terminal_clears_any_open_session() {
        let (ctx, terminal, ..) = ctx_with_terminal_product_and_accounts();
        let mut state = State {
            open_session_id: Some(42),
            ..Default::default()
        };

        update(
            &mut state,
            Message::TerminalSelected(terminal.clone()),
            &ctx,
        );

        assert_eq!(state.terminal, Some(terminal));
        assert!(state.open_session_id.is_none());
    }

    #[test]
    fn opening_a_session_without_a_terminal_sets_an_error() {
        let (ctx, ..) = ctx_with_terminal_product_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::OpenSession, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a terminal first"));
    }

    #[test]
    fn opening_a_session_with_a_terminal_chosen_succeeds() {
        let (ctx, terminal, ..) = ctx_with_terminal_product_and_accounts();
        let mut state = State {
            terminal: Some(terminal),
            ..Default::default()
        };

        update(&mut state, Message::OpenSession, &ctx);

        assert!(state.error.is_none());
        assert!(state.open_session_id.is_some());
        assert_eq!(state.message.as_deref(), Some("Session opened."));
    }

    #[test]
    fn ringing_up_a_sale_without_an_open_session_sets_an_error() {
        let (ctx, terminal, product, revenue, cash) = ctx_with_terminal_product_and_accounts();
        let mut state = State {
            terminal: Some(terminal),
            product: Some(product),
            revenue_account: Some(revenue),
            cash_account: Some(cash),
            price_input: "50".into(),
            ..Default::default()
        };

        update(&mut state, Message::RingUpCashSale, &ctx);

        assert!(state.error.is_some());
    }

    #[test]
    fn ringing_up_a_sale_with_a_non_numeric_price_sets_an_error() {
        let (ctx, terminal, product, revenue, cash) = ctx_with_terminal_product_and_accounts();
        let mut state = State {
            terminal: Some(terminal),
            product: Some(product),
            revenue_account: Some(revenue),
            cash_account: Some(cash),
            price_input: "not a number".into(),
            ..Default::default()
        };
        update(&mut state, Message::OpenSession, &ctx);

        update(&mut state, Message::RingUpCashSale, &ctx);

        assert_eq!(state.error.as_deref(), Some("price is not a valid number"));
    }

    #[test]
    fn ringing_up_a_valid_cash_sale_succeeds_and_clears_the_price() {
        let (ctx, terminal, product, revenue, cash) = ctx_with_terminal_product_and_accounts();
        let mut state = State {
            terminal: Some(terminal),
            product: Some(product),
            revenue_account: Some(revenue),
            cash_account: Some(cash),
            price_input: "50".into(),
            ..Default::default()
        };
        update(&mut state, Message::OpenSession, &ctx);

        update(&mut state, Message::RingUpCashSale, &ctx);

        assert!(state.error.is_none());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("rung up"));
        assert!(state.price_input.is_empty());
    }
}
