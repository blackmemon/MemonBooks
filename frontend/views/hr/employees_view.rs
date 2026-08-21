//! List employees and add new ones.

use chrono::Utc;

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::hr::employees::Employee;
use crate::backend::repositories::organization_repository::OrganizationRepository;
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
    let repo = OrganizationRepository::new(ctx.db.clone());
    match repo.list_employees_for_company(ctx.company_id) {
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
            let repo = OrganizationRepository::new(ctx.db.clone());
            match repo.create_employee(&employee) {
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
