//! List the chart of accounts, add new accounts, edit or delete
//! existing ones (singly or in bulk), and export a selection to CSV.

use std::collections::HashSet;

use iced::widget::{button, column, container, mouse_area, opaque, pick_list, row, scrollable, stack, text, text_input, Space};
use iced::{Background, Color, Element, Length};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::reference_code;
use crate::backend::core::validation::Validate;
use crate::backend::services::accounting_service::AccountingService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::borders;
use crate::frontend::views::Ctx;

const ACCOUNT_TYPES: [AccountType; 5] = [
    AccountType::Asset,
    AccountType::Liability,
    AccountType::Equity,
    AccountType::Revenue,
    AccountType::Expense,
];

#[derive(Debug, Clone, Default)]
pub struct State {
    accounts: Vec<Account>,
    code_input: String,
    name_input: String,
    type_input: Option<AccountType>,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// account instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    /// Whether the "Add Account" slide-over panel is open. Hidden by
    /// default; opens only via the header's "Add account" action and
    /// closes on Save, Cancel, or clicking outside it.
    add_panel_open: bool,
    /// The reference code shown (read-only) in the open panel --
    /// generated fresh the moment the panel opens, and carried through
    /// unchanged into the saved account so what the user saw is exactly
    /// what gets persisted (`Account::new` would otherwise generate its
    /// own, different, one internally). A cancelled value is simply
    /// discarded, never reused, since opening the panel again always
    /// regenerates it.
    add_reference_code: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CodeChanged(String),
    NameChanged(String),
    TypeSelected(AccountType),
    Create,
    OpenAddPanel,
    CloseAddPanel,
    EditRow(i64),
    CancelEdit,
    RowSelected(i64, bool),
    SelectAllChanged(Vec<i64>, bool),
    ClearSelection,
    BulkDelete,
    BulkExportCsv,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
}

pub fn load(ctx: &Ctx) -> State {
    let service = AccountingService::new(ctx.db.clone());
    match service.list_accounts(ctx.company_id) {
        Ok(accounts) => State { accounts, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CodeChanged(value) => state.code_input = value,
        Message::NameChanged(value) => state.name_input = value,
        Message::TypeSelected(account_type) => state.type_input = Some(account_type),
        Message::Create => {
            state.message = None;
            state.error = None;
            let Some(account_type) = state.type_input else {
                state.error = Some("choose an account type".into());
                return;
            };

            if let Some(id) = state.editing_id {
                let Some(existing) = state.accounts.iter().find(|a| a.id == Some(id)).cloned() else {
                    state.error = Some("that account no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let account = Account { code: state.code_input.clone(), name: state.name_input.clone(), account_type, ..existing };
                if let Err(err) = account.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = AccountingService::new(ctx.db.clone());
                match service.update_account(&account) {
                    Ok(()) => {
                        if let Some(slot) = state.accounts.iter_mut().find(|a| a.id == Some(id)) {
                            *slot = account;
                        }
                        state.accounts.sort_by(|a, b| a.code.cmp(&b.code));
                        state.editing_id = None;
                        state.code_input.clear();
                        state.name_input.clear();
                        state.type_input = None;
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut account = Account::new(ctx.company_id, state.code_input.clone(), state.name_input.clone(), account_type);
            // Overrides whatever `Account::new` generated internally --
            // the panel already showed `add_reference_code` to the user
            // as the account's reference code, so that's the value that
            // must actually be saved, not a second, different one. Only
            // when the panel actually supplied one: skipping straight to
            // `Create` without opening it first (every pre-panel test,
            // and any future caller that does the same) must keep
            // `Account::new`'s own generated code rather than collide
            // every such account on the same blank string against the
            // `reference_code` unique index.
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
                    state.accounts.sort_by(|a, b| a.code.cmp(&b.code));
                    state.code_input.clear();
                    state.name_input.clear();
                    state.type_input = None;
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
            state.add_reference_code = reference_code::generate_code("coa");
            state.code_input.clear();
            state.name_input.clear();
            state.type_input = None;
        }
        Message::CloseAddPanel => {
            state.add_panel_open = false;
            state.code_input.clear();
            state.name_input.clear();
            state.type_input = None;
            state.error = None;
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(account) = state.accounts.iter().find(|a| a.id == Some(id)) {
                state.editing_id = Some(id);
                state.code_input = account.code.clone();
                state.name_input = account.name.clone();
                state.type_input = Some(account.account_type);
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.code_input.clear();
            state.name_input.clear();
            state.type_input = None;
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
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&Account> = state.accounts.iter().filter(|a| state.selected.contains(&a.id.unwrap_or_default())).collect();
            let headers = ["Code", "Name", "Type", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|a| vec![a.code.clone(), a.name.clone(), a.account_type.as_str().to_string(), if a.is_active { "Active".to_string() } else { "Inactive".to_string() }])
                .collect();
            let path = ctx.export_path("chart_of_accounts_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} account(s) to {}", selected.len(), path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, and "Filters" / "Sort by" are
        // rendered by the shared toolbar component now (Section 6) --
        // wiring real tab/filter/sort behavior is follow-up work, not
        // part of this pass.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked => {}
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add account" };
    let header_action = if state.editing_id.is_some() { Message::Create } else { Message::OpenAddPanel };
    let header = page_header::view(theme, "Chart of Accounts", "Every general ledger account on this company's books.", header_label, header_action);

    let toolbar = toolbar::view(
        theme,
        &["All Accounts"],
        0,
        Message::TabSelected,
        &state.search,
        "Search accounts",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Account list", "Every account, grouped by code order.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Account> =
        state.accounts.iter().filter(|account| query.is_empty() || account.name.to_lowercase().contains(&query) || account.code.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Code", Length::FillPortion(1)).sorted(SortDirection::Ascending),
        ColumnDef::new("Name", Length::FillPortion(3)),
        ColumnDef::new("Type", Length::FillPortion(1)),
    ];

    let rows = filtered.iter().map(|account| Row::new(vec![Cell::text(account.code.clone()), Cell::text(account.name.clone()), Cell::text(account.account_type.as_str())])).collect();

    let row_ids: Vec<i64> = filtered.iter().map(|a| a.id.unwrap_or_default()).collect();
    let select_ids = row_ids.clone();
    let selected_flags: Vec<bool> = row_ids.iter().map(|id| state.selected.contains(id)).collect();
    let edit_ids = row_ids.clone();
    let edit_ids_primary = row_ids.clone();

    let table = data_table::view(
        theme,
        columns,
        rows,
        Some(selected_flags),
        move |index, checked| Message::RowSelected(row_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::EditRow(edit_ids[index]),
        move |index| Message::EditRow(edit_ids_primary[index]),
    );

    let mut content = column![header, toolbar, heading].spacing(20).padding(24).width(Length::Fill);

    if !state.selected.is_empty() {
        content = content.push(bulk_action_bar::view(theme, state.selected.len(), Message::BulkDelete, Message::BulkExportCsv, Message::ClearSelection));
    }

    content = content.push(table);

    // Editing stays an inline form below the table, unchanged -- only
    // adding a new account moved to the slide-over panel below.
    if state.editing_id.is_some() {
        let form = column![
            text("Edit account").size(13),
            text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
            text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
            pick_list(ACCOUNT_TYPES, state.type_input, Message::TypeSelected).placeholder("Type"),
            iced::widget::row![
                button(text("Save changes")).on_press(Message::Create).padding(8),
                button(text("Cancel")).on_press(Message::CancelEdit).padding(8),
            ]
            .spacing(8),
        ]
        .spacing(8);
        content = content.push(form);
    }

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    let base: Element<'_, Message> = container(scrollable(content).height(Length::Fill)).width(Length::Fill).into();

    if !state.add_panel_open {
        return base;
    }

    let backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_iced_theme: &iced::Theme| container::Style {
                background: Some(Background::Color(Color { a: 0.35, ..Color::BLACK })),
                ..container::Style::default()
            }),
    )
    .on_press(Message::CloseAddPanel);

    let panel_row = row![Space::with_width(Length::Fill), add_panel(state, theme)];

    stack(vec![base, backdrop.into(), opaque(panel_row)]).into()
}

/// The "Add Account" slide-over: hidden until `OpenAddPanel`, anchored
/// to the right edge via the `Space` pushed ahead of it in `view`'s
/// `panel_row`. `opaque` (wrapping this in `view`) keeps a click here
/// from falling through to the backdrop underneath and closing the
/// panel it's supposed to be interacting with.
fn add_panel(state: &State, theme: Theme) -> Element<'_, Message> {
    let content = column![
        text("Add an account").size(15),
        column![text("Reference Code").size(12).color(theme.neutral_front()), text(state.add_reference_code.clone()).size(14)].spacing(4),
        column![
            text("Account Number").size(12).color(theme.neutral_front()),
            text_input("e.g. 1000", &state.code_input).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
        ]
        .spacing(4),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        pick_list(ACCOUNT_TYPES, state.type_input, Message::TypeSelected).placeholder("Type"),
        iced::widget::row![
            button(text("Save")).on_press(Message::Create).padding(8),
            button(text("Cancel")).on_press(Message::CloseAddPanel).padding(8),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .padding(24)
    .width(Length::Fixed(380.0))
    .height(Length::Fill);

    container(content)
        .style(move |_iced_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(theme.surface())),
            border: borders::card(theme, borders::RADIUS_LG),
            ..container::Style::default()
        })
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

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_chart_of_accounts_view_test_{:?}", std::thread::current().id())), ..ctx }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::CodeChanged("1000".into()), &ctx);
        update(&mut state, Message::NameChanged("Cash".into()), &ctx);
        update(&mut state, Message::TypeSelected(AccountType::Asset), &ctx);

        assert_eq!(state.code_input, "1000");
        assert_eq!(state.name_input, "Cash");
        assert_eq!(state.type_input, Some(AccountType::Asset));
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
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].code, "1000");
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
        assert!(state.type_input.is_none());
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
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.accounts[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.code_input, "1000");
        assert_eq!(state.name_input, "Cash");
        assert_eq!(state.type_input, Some(AccountType::Asset));
    }

    #[test]
    fn saving_an_edit_updates_the_existing_account_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.accounts[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Petty Cash".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].name, "Petty Cash");

        let service = AccountingService::new(ctx.db.clone());
        let reloaded = service.list_accounts(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].name, "Petty Cash");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_account() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.accounts[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.accounts[0].name, "Cash");
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
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        state.code_input = "1010".into();
        state.name_input = "Petty Cash".into();
        state.type_input = Some(AccountType::Asset);
        update(&mut state, Message::Create, &ctx);
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
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        state.code_input = "3000".into();
        state.name_input = "Owner's Capital".into();
        state.type_input = Some(AccountType::Equity);
        update(&mut state, Message::Create, &ctx);
        state.code_input = "1010".into();
        state.name_input = "Petty Cash".into();
        state.type_input = Some(AccountType::Asset);
        update(&mut state, Message::Create, &ctx);

        let cash_id = state.accounts.iter().find(|a| a.code == "1000").unwrap().id.unwrap();
        let capital_id = state.accounts.iter().find(|a| a.code == "3000").unwrap().id.unwrap();
        let free_id = state.accounts.iter().find(|a| a.code == "1010").unwrap().id.unwrap();

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
        let mut state = State { code_input: "stale".into(), name_input: "Stale".into(), type_input: Some(AccountType::Asset), ..Default::default() };

        update(&mut state, Message::OpenAddPanel, &ctx);

        assert!(state.add_panel_open);
        assert!(state.add_reference_code.starts_with("coa-"));
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
        assert!(state.type_input.is_none());
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
        update(&mut state, Message::TypeSelected(AccountType::Asset), &ctx);
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(!state.add_panel_open, "saving must close the panel");
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].code, "1000");
        assert_eq!(state.accounts[0].name, "Cash");
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
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        state.code_input = "1010".into();
        state.name_input = "Petty Cash".into();
        state.type_input = Some(AccountType::Asset);
        update(&mut state, Message::Create, &ctx);
        let export_id = state.accounts.iter().find(|a| a.code == "1000").unwrap().id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("chart_of_accounts_export.csv")).unwrap();
        assert!(contents.contains("Cash") && !contents.contains("Petty Cash"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
