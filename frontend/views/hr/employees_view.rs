//! List employees, add new ones, edit or delete existing ones (singly
//! or in bulk), and export a selection to CSV.

use std::collections::HashSet;

use chrono::Utc;

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::hr::employees::Employee;
use crate::backend::services::organization_service::OrganizationService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::colors::Theme;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    employees: Vec<Employee>,
    code_input: String,
    name_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// employee instead of creating a new one -- `Message::Create` does
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
    let service = OrganizationService::new(ctx.db.clone());
    match service.list_employees(ctx.company_id) {
        Ok(employees) => State { employees, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CodeChanged(value) => state.code_input = value,
        Message::NameChanged(value) => state.name_input = value,
        Message::Create => {
            state.message = None;
            state.error = None;

            if let Some(id) = state.editing_id {
                let Some(existing) = state.employees.iter().find(|e| e.id == Some(id)).cloned() else {
                    state.error = Some("that employee no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let employee = Employee { employee_code: state.code_input.clone(), full_name: state.name_input.clone(), ..existing };
                if let Err(err) = employee.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = OrganizationService::new(ctx.db.clone());
                match service.update_employee(&employee) {
                    Ok(()) => {
                        if let Some(slot) = state.employees.iter_mut().find(|e| e.id == Some(id)) {
                            *slot = employee;
                        }
                        state.editing_id = None;
                        state.code_input.clear();
                        state.name_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let today = Utc::now().date_naive();
            let employee = Employee::new(ctx.company_id, state.code_input.clone(), state.name_input.clone(), today);
            if let Err(err) = employee.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let service = OrganizationService::new(ctx.db.clone());
            match service.create_employee(&employee) {
                Ok(created) => {
                    state.employees.push(created);
                    state.code_input.clear();
                    state.name_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(employee) = state.employees.iter().find(|e| e.id == Some(id)) {
                state.editing_id = Some(id);
                state.code_input = employee.employee_code.clone();
                state.name_input = employee.full_name.clone();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.code_input.clear();
            state.name_input.clear();
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
            let service = OrganizationService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_employee(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.employees.retain(|e| e.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("employee #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} employee(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&Employee> = state.employees.iter().filter(|e| state.selected.contains(&e.id.unwrap_or_default())).collect();
            let headers = ["Code", "Full Name", "Hired", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|e| vec![e.employee_code.clone(), e.full_name.clone(), e.hire_date.to_string(), if e.is_active { "Active".to_string() } else { "Inactive".to_string() }])
                .collect();
            let path = ctx.export_path("employees_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} employee(s) to {}", selected.len(), path.display())),
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
    let header_label = if state.editing_id.is_some() { "Save changes" } else { "Add employee" };
    let header = page_header::view(theme, "Employees", "Everyone on this company's payroll.", header_label, Message::Create);

    let toolbar = toolbar::view(
        theme,
        &["All Employees"],
        0,
        Message::TabSelected,
        &state.search,
        "Search employees",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Employee list", "Every employee on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Employee> =
        state.employees.iter().filter(|employee| query.is_empty() || employee.full_name.to_lowercase().contains(&query) || employee.employee_code.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Employee", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Hired", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|employee| Row::new(vec![Cell::person(employee.full_name.clone(), employee.employee_code.clone()), Cell::date(employee.hire_date.to_string())]))
        .collect();

    let row_ids: Vec<i64> = filtered.iter().map(|e| e.id.unwrap_or_default()).collect();
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

    let form_heading = if state.editing_id.is_some() { "Edit employee" } else { "Add an employee" };
    let mut form = column![
        text(form_heading).size(13),
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
        text_input("Full name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
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
        form = form.push(button(text("Add employee")).on_press(Message::Create).padding(8));
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
    use crate::backend::repositories::user_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_employees_view_test_{:?}", std::thread::current().id())), ..ctx }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let mut state = State::default();

        update(&mut state, Message::CodeChanged("EMP-001".into()), &ctx_with_company());
        update(&mut state, Message::NameChanged("Jane Doe".into()), &ctx_with_company());

        assert_eq!(state.code_input, "EMP-001");
        assert_eq!(state.name_input, "Jane Doe");
    }

    #[test]
    fn creating_with_a_blank_name_is_rejected_before_reaching_the_service() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "EMP-001".into(), name_input: "".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_some());
        assert!(state.employees.is_empty());
    }

    #[test]
    fn creating_a_valid_employee_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "EMP-001".into(), name_input: "Jane Doe".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.employees.len(), 1);
        assert_eq!(state.employees[0].full_name, "Jane Doe");
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("jane".into()), &ctx);

        assert_eq!(state.search, "jane");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_employee() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "EMP-001".into(), name_input: "Jane Doe".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.employees[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.code_input, "EMP-001");
        assert_eq!(state.name_input, "Jane Doe");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_employee_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "EMP-001".into(), name_input: "Jane Doe".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.employees[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Jane Smith".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.employees.len(), 1);
        assert_eq!(state.employees[0].full_name, "Jane Smith");

        let service = OrganizationService::new(ctx.db.clone());
        let reloaded = service.list_employees(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].full_name, "Jane Smith");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_employee() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "EMP-001".into(), name_input: "Jane Doe".into(), ..Default::default() };
        update(&mut state, Message::Create, &ctx);
        let id = state.employees[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.employees[0].full_name, "Jane Doe");
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
    fn bulk_delete_removes_every_selected_unreferenced_employee() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::CodeChanged("EMP-001".into()), &ctx);
        state.name_input = "Jane Doe".into();
        update(&mut state, Message::Create, &ctx);
        state.code_input = "EMP-002".into();
        state.name_input = "John Smith".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.employees.iter().map(|e| e.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.employees.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    /// A selection that includes an employee still referenced by a
    /// salary structure: the unreferenced one is deleted, the referenced
    /// one is skipped with a clear per-record error in the summary --
    /// bulk delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_employee_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::hr::salary::{SalaryComponent, SalaryStructure};

        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::CodeChanged("EMP-001".into()), &ctx);
        state.name_input = "Jane Doe".into();
        update(&mut state, Message::Create, &ctx);
        state.code_input = "EMP-002".into();
        state.name_input = "John Smith".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.employees.iter().find(|e| e.employee_code == "EMP-001").unwrap().id.unwrap();
        let free_id = state.employees.iter().find(|e| e.employee_code == "EMP-002").unwrap().id.unwrap();

        let mut structure = SalaryStructure::new(referenced_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        structure.components.push(SalaryComponent::addition("Base Salary", rust_decimal_macros::dec!(1000)));
        let service = OrganizationService::new(ctx.db.clone());
        service.create_salary_structure(&structure).unwrap();

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.employees.len(), 1);
        assert_eq!(state.employees[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_employees() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::CodeChanged("EMP-001".into()), &ctx);
        state.name_input = "Jane Doe".into();
        update(&mut state, Message::Create, &ctx);
        state.code_input = "EMP-002".into();
        state.name_input = "John Smith".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.employees.iter().find(|e| e.employee_code == "EMP-001").unwrap().id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("employees_export.csv")).unwrap();
        assert!(contents.contains("Jane Doe") && !contents.contains("John Smith"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
