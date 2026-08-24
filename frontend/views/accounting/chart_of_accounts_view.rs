//! List the chart of accounts, add new accounts, edit or delete
//! existing ones (singly or in bulk), and export a selection to CSV.

use std::collections::HashSet;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::validation::Validate;
use crate::backend::services::accounting_service::AccountingService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::colors;
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
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CodeChanged(String),
    NameChanged(String),
    TypeSelected(AccountType),
    Create,
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

            let account = Account::new(ctx.company_id, state.code_input.clone(), state.name_input.clone(), account_type);
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
                }
                Err(err) => state.error = Some(err.to_string()),
            }
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

pub fn view(state: &State) -> Element<'_, Message> {
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add account" };
    let header = page_header::view("Chart of Accounts", "Every general ledger account on this company's books.", header_label, Message::Create);

    let toolbar = toolbar::view(
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

    let heading = section_heading::view("Account list", "Every account, grouped by code order.");

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
        columns,
        rows,
        Some(selected_flags),
        move |index, checked| Message::RowSelected(row_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::EditRow(edit_ids[index]),
        move |index| Message::EditRow(edit_ids_primary[index]),
    );

    let form_heading = if state.editing_id.is_some() { "Edit account" } else { "Add an account" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        pick_list(ACCOUNT_TYPES, state.type_input, Message::TypeSelected).placeholder("Type"),
    ]
    .spacing(8);

    if state.editing_id.is_some() {
        form = form.push(
            iced::widget::row![
                button(text("Save changes")).on_press(Message::Create).padding(8),
                button(text("Cancel")).on_press(Message::CancelEdit).padding(8),
            ]
            .spacing(8),
        );
    } else {
        form = form.push(button(text("Add account")).on_press(Message::Create).padding(8));
    }

    let mut content = column![header, toolbar, heading].spacing(20).padding(24).width(Length::Fill);

    if !state.selected.is_empty() {
        content = content.push(bulk_action_bar::view(state.selected.len(), Message::BulkDelete, Message::BulkExportCsv, Message::ClearSelection));
    }

    content = content.push(table).push(form);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(scrollable(content).height(Length::Fill)).width(Length::Fill).into()
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
