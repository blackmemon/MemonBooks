//! List customers, add new ones, edit or delete existing ones (singly
//! or in bulk), and export a selection to CSV.

use std::collections::HashSet;

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::parties::customers::Customer;
use crate::backend::services::party_service::PartyService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    customers: Vec<Customer>,
    name_input: String,
    email_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// customer instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    EmailChanged(String),
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
    let service = PartyService::new(ctx.db.clone());
    match service.list_customers(ctx.company_id) {
        Ok(customers) => State { customers, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::EmailChanged(value) => state.email_input = value,
        Message::Create => {
            state.message = None;
            state.error = None;

            if let Some(id) = state.editing_id {
                let Some(existing) = state.customers.iter().find(|c| c.id == Some(id)).cloned() else {
                    state.error = Some("that customer no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let customer = Customer {
                    name: state.name_input.clone(),
                    email: if state.email_input.trim().is_empty() { None } else { Some(state.email_input.clone()) },
                    ..existing
                };
                if let Err(err) = customer.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = PartyService::new(ctx.db.clone());
                match service.update_customer(&customer) {
                    Ok(()) => {
                        if let Some(slot) = state.customers.iter_mut().find(|c| c.id == Some(id)) {
                            *slot = customer;
                        }
                        state.editing_id = None;
                        state.name_input.clear();
                        state.email_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut customer = Customer::new(ctx.company_id, state.name_input.clone());
            if !state.email_input.trim().is_empty() {
                customer.email = Some(state.email_input.clone());
            }
            if let Err(err) = customer.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let service = PartyService::new(ctx.db.clone());
            match service.create_customer(&customer) {
                Ok(created) => {
                    state.customers.push(created);
                    state.name_input.clear();
                    state.email_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(customer) = state.customers.iter().find(|c| c.id == Some(id)) {
                state.editing_id = Some(id);
                state.name_input = customer.name.clone();
                state.email_input = customer.email.clone().unwrap_or_default();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.name_input.clear();
            state.email_input.clear();
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
            let service = PartyService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_customer(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.customers.retain(|c| c.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("customer #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} customer(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&Customer> = state.customers.iter().filter(|c| state.selected.contains(&c.id.unwrap_or_default())).collect();
            let headers = ["Name", "Email", "Phone", "Payment Terms (days)", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|c| {
                    vec![
                        c.name.clone(),
                        c.email.clone().unwrap_or_default(),
                        c.phone.clone().unwrap_or_default(),
                        c.payment_terms_days.to_string(),
                        if c.is_active { "Active".to_string() } else { "Inactive".to_string() },
                    ]
                })
                .collect();
            let path = ctx.export_path("customers_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} customer(s) to {}", selected.len(), path.display())),
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
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add customer" };
    let header = page_header::view(theme, "Customers", "Everyone you invoice, in one place.", header_label, Message::Create);

    let toolbar = toolbar::view(
        theme,
        &["All Customers"],
        0,
        Message::TabSelected,
        &state.search,
        "Search customers",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Customer list", "Every customer on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Customer> = state
        .customers
        .iter()
        .filter(|customer| query.is_empty() || customer.name.to_lowercase().contains(&query) || customer.email.as_deref().unwrap_or_default().to_lowercase().contains(&query))
        .collect();

    let columns = vec![
        ColumnDef::new("Customer", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Phone", Length::FillPortion(2)),
        ColumnDef::new("Terms", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|customer| {
            Row::new(vec![
                Cell::person(customer.name.clone(), customer.email.clone().unwrap_or_default()),
                Cell::text(customer.phone.clone().unwrap_or_else(|| "--".to_string())),
                Cell::text(format!("{} days", customer.payment_terms_days)),
                if customer.is_active { Cell::status("Active", BadgeTone::Positive) } else { Cell::status("Inactive", BadgeTone::Neutral) },
            ])
        })
        .collect();

    let row_ids: Vec<i64> = filtered.iter().map(|c| c.id.unwrap_or_default()).collect();
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

    let form_heading = if state.editing_id.is_some() { "Edit customer" } else { "Add a customer" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("Customer name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        text_input("Email (optional)", &state.email_input).on_input(Message::EmailChanged).width(Length::Fill).padding(8),
    ]
    .spacing(8);

    if state.editing_id.is_some() {
        form = form.push(iced::widget::row![
            button(text("Save changes")).on_press(Message::Create).padding(8),
            button(text("Cancel")).on_press(Message::CancelEdit).padding(8),
        ].spacing(8));
    } else {
        form = form.push(button(text("Add customer")).on_press(Message::Create).padding(8));
    }

    let mut content = column![header, toolbar, heading].spacing(20).padding(24).width(Length::Fill);

    if !state.selected.is_empty() {
        content = content.push(bulk_action_bar::view(theme, state.selected.len(), Message::BulkDelete, Message::BulkExportCsv, Message::ClearSelection));
    }

    content = content.push(table).push(form);

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
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_customers_view_test_{:?}", std::thread::current().id())), ..ctx }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::EmailChanged("ar@acmeretail.test".into()), &ctx);

        assert_eq!(state.name_input, "Acme Retail");
        assert_eq!(state.email_input, "ar@acmeretail.test");
    }

    #[test]
    fn creating_with_a_blank_name_is_rejected_before_reaching_the_service() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_some());
        assert!(state.customers.is_empty());
    }

    #[test]
    fn creating_with_a_blank_email_leaves_it_unset() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Retail".into(), email_input: "".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.customers[0].email, None);
    }

    #[test]
    fn creating_a_valid_customer_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Retail".into(), email_input: "ar@acmeretail.test".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.customers.len(), 1);
        assert_eq!(state.customers[0].email.as_deref(), Some("ar@acmeretail.test"));
        assert!(state.name_input.is_empty());
        assert!(state.email_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("acme".into()), &ctx);

        assert_eq!(state.search, "acme");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_customer() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Retail".into(), email_input: "ar@acmeretail.test".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.name_input, "Acme Retail");
        assert_eq!(state.email_input, "ar@acmeretail.test");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_customer_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Retail".into(), email_input: "".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Acme Retail Renamed".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.customers.len(), 1);
        assert_eq!(state.customers[0].name, "Acme Retail Renamed");
        assert_eq!(state.customers[0].id, Some(id));

        // Persisted, not just updated in the in-memory list.
        let service = PartyService::new(ctx.db.clone());
        let reloaded = service.list_customers(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].name, "Acme Retail Renamed");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_customer() {
        let ctx = ctx_with_company();
        let mut state = State { name_input: "Acme Retail".into(), email_input: "".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.customers[0].name, "Acme Retail");
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
    fn bulk_delete_removes_every_selected_unreferenced_customer() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.customers.iter().map(|c| c.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.customers.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes a customer still referenced by an
    /// invoice: the unreferenced one is deleted, the referenced one is
    /// skipped with a clear per-record error in the summary -- bulk
    /// delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_customer_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::sales_repository::{self, SalesRepository};
        use crate::backend::sales::invoices::{Invoice, InvoiceLine};
        use crate::backend::services::sales_service::SalesService;

        let ctx = ctx_with_company();
        ctx.db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        ctx.db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        ctx.db.run_migrations(sales_repository::MIGRATIONS).unwrap();

        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.customers[0].id.unwrap();
        let free_id = state.customers[1].id.unwrap();

        let accounting = AccountingRepository::new(ctx.db.clone());
        accounting.create_fiscal_period(&FiscalPeriod::new(ctx.company_id, "Open Period", chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap())).unwrap();
        let ar = accounting.create_account(&Account::new(ctx.company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
        let revenue = accounting.create_account(&Account::new(ctx.company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();
        let sales = SalesRepository::new(ctx.db.clone());
        let mut invoice = Invoice::new(ctx.company_id, referenced_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(), chrono::NaiveDate::from_ymd_opt(2026, 2, 4).unwrap());
        invoice.lines.push(InvoiceLine::new(revenue.id.unwrap(), "Consulting", rust_decimal_macros::dec!(1), rust_decimal_macros::dec!(500)));
        let created = sales.create_invoice(&invoice).unwrap();
        sales.post_invoice(created.id.unwrap(), ar.id.unwrap()).unwrap();
        let _ = SalesService::new(ctx.db.clone());

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.customers.len(), 1);
        assert_eq!(state.customers[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_customers() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.customers[0].id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("customers_export.csv")).unwrap();
        assert!(contents.contains("Acme Retail"));
        assert!(!contents.contains("Beta Supplies"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
