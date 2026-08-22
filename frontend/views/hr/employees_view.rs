//! List employees and add new ones.

use chrono::Utc;

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::hr::employees::Employee;
use crate::backend::services::organization_service::OrganizationService;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    employees: Vec<Employee>,
    code_input: String,
    name_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CodeChanged(String),
    NameChanged(String),
    Create,
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
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Code").width(Length::Fixed(100.0)),
        text("Name").width(Length::Fill),
        text("Hired").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for employee in &state.employees {
        list = list.push(row![
            text(employee.employee_code.clone()).width(Length::Fixed(100.0)),
            text(employee.full_name.clone()).width(Length::Fill),
            text(employee.hire_date.to_string()).width(Length::Fixed(100.0)),
        ]);
    }

    let form = row![
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fixed(100.0)).padding(6),
        text_input("Full name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        button(text("Add employee")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Employees").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
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
}
