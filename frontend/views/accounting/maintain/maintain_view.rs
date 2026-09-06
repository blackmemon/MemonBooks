use iced::widget::pick_list;
use iced::{Element, Length};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::spacing;
use crate::frontend::views::Ctx;
use super::{bank_of_accounts_view, chart_of_accounts_view, customers_view, vendors_view, warehouse_view};

/// The five record types grouped behind the sidebar's "Maintain" entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    ChartOfAccounts,
    Vendors,
    Warehouses,
    BankAccounts,
    Customers,
}

pub const SECTIONS: [Section; 5] = [Section::ChartOfAccounts, Section::Vendors, Section::Warehouses, Section::BankAccounts, Section::Customers];

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Section::ChartOfAccounts => "Chart of Account",
            Section::Vendors => "Vendor",
            Section::Warehouses => "Warehouse",
            Section::BankAccounts => "Bank Accounts",
            Section::Customers => "Customers",
        })
    }
}

#[derive(Debug, Default)]
pub struct State {
    selected: Section,
    chart_of_accounts: chart_of_accounts_view::State,
    vendors: vendors_view::State,
    warehouses: warehouse_view::State,
    bank_accounts: bank_of_accounts_view::State,
    customers: customers_view::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Section),
    ChartOfAccounts(chart_of_accounts_view::Message),
    Vendors(vendors_view::Message),
    Warehouses(warehouse_view::Message),
    BankAccounts(bank_of_accounts_view::Message),
    Customers(customers_view::Message),
}

pub fn load(ctx: &Ctx) -> State {
    let mut state = State::default();
    let selected = state.selected;
    load_section(&mut state, selected, ctx);
    state
}

fn load_section(state: &mut State, section: Section, ctx: &Ctx) {
    match section {
        Section::ChartOfAccounts => state.chart_of_accounts = chart_of_accounts_view::load(ctx),
        Section::Vendors => state.vendors = vendors_view::load(ctx),
        Section::Warehouses => state.warehouses = warehouse_view::load(ctx),
        Section::BankAccounts => state.bank_accounts = bank_of_accounts_view::load(ctx),
        Section::Customers => state.customers = customers_view::load(ctx),
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Select(section) => {
            state.selected = section;
            load_section(state, section, ctx);
        }
        Message::ChartOfAccounts(inner) => chart_of_accounts_view::update(&mut state.chart_of_accounts, inner, ctx),
        Message::Vendors(inner) => vendors_view::update(&mut state.vendors, inner, ctx),
        Message::Warehouses(inner) => warehouse_view::update(&mut state.warehouses, inner, ctx),
        Message::BankAccounts(inner) => bank_of_accounts_view::update(&mut state.bank_accounts, inner, ctx),
        Message::Customers(inner) => customers_view::update(&mut state.customers, inner, ctx),
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let switcher: Element<'_, Message> = pick_list(SECTIONS, Some(state.selected), Message::Select).style(dropdown::style(theme)).padding(spacing::SM).width(Length::Fixed(220.0)).into();

    match state.selected {
        Section::ChartOfAccounts => chart_of_accounts_view::page_header_view_with_switcher(&state.chart_of_accounts, theme, switcher, Message::ChartOfAccounts),
        Section::Vendors => vendors_view::page_header_view_with_switcher(&state.vendors, theme, switcher, Message::Vendors),
        Section::Warehouses => warehouse_view::page_header_view_with_switcher(&state.warehouses, theme, switcher, Message::Warehouses),
        Section::BankAccounts => bank_of_accounts_view::page_header_view_with_switcher(&state.bank_accounts, theme, switcher, Message::BankAccounts),
        Section::Customers => customers_view::page_header_view_with_switcher(&state.customers, theme, switcher, Message::Customers),
    }
}