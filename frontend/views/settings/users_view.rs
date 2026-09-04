//! Admin-only user management: list every user account, create new
//! ones (with a generated temporary password shown once), edit a
//! user's role or active status, and trigger an admin password reset.
//! Reachable only via the sidebar's "Users" item, which `src/main.rs`
//! hides unless the signed-in user's role has `users.manage` -- and
//! every mutating action here is re-checked at the service layer
//! regardless (`AuthService::require_permission`), so hiding the nav
//! item is a UX nicety, not the actual security boundary.

use iced::widget::{button, checkbox, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::core::roles::Role;
use crate::backend::core::users::User;
use crate::backend::services::auth_service::AuthService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::styles::checkbox as checkbox_style;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, PartialEq)]
pub struct RoleChoice {
    id: i64,
    label: String,
}

impl std::fmt::Display for RoleChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    users: Vec<User>,
    roles: Vec<Role>,
    role_choices: Vec<RoleChoice>,
    username_input: String,
    display_name_input: String,
    role_input: Option<RoleChoice>,
    /// `Some(id)` while the edit panel is open for an existing user --
    /// only role and active status are editable here (not username or
    /// password: renaming an account is out of scope, and password
    /// changes go through the dedicated Reset Password action instead).
    editing_id: Option<i64>,
    edit_role_input: Option<RoleChoice>,
    edit_is_active: bool,
    /// A freshly generated temporary password, shown exactly once right
    /// after creating a user or resetting one's password -- it's never
    /// recoverable after this, since only the hash is stored.
    generated_password: Option<String>,
    generated_password_for: String,
    search: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    UsernameChanged(String),
    DisplayNameChanged(String),
    RoleSelected(RoleChoice),
    CreateUser,
    DismissGeneratedPassword,
    EditRow(i64),
    EditRoleSelected(RoleChoice),
    EditActiveToggled(bool),
    SaveEdit,
    CancelEdit,
    ResetPasswordForRow(i64),
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    /// Fired by DataTable's checkbox-column callbacks, which this view
    /// never enables (`selected` is always `None` below) -- kept only
    /// because the callbacks are required by the shared signature.
    Noop,
}

pub fn load(ctx: &Ctx) -> State {
    let service = AuthService::new(ctx.db.clone());
    let users = service.list_users().unwrap_or_default();
    let roles = service.list_roles().unwrap_or_default();
    let role_choices = roles.iter().filter_map(|r| r.id.map(|id| RoleChoice { id, label: r.name.clone() })).collect();

    State { users, roles, role_choices, ..Default::default() }
}

fn role_name(roles: &[Role], role_id: i64) -> String {
    roles.iter().find(|r| r.id == Some(role_id)).map(|r| r.name.clone()).unwrap_or_else(|| format!("Role #{role_id}"))
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx, acting_user: &User) {
    match message {
        Message::UsernameChanged(value) => state.username_input = value,
        Message::DisplayNameChanged(value) => state.display_name_input = value,
        Message::RoleSelected(choice) => state.role_input = Some(choice),
        Message::CreateUser => {
            state.message = None;
            state.error = None;
            let Some(role) = &state.role_input else {
                state.error = Some("choose a role".into());
                return;
            };
            let service = AuthService::new(ctx.db.clone());
            match service.create_user(acting_user, &state.username_input, &state.display_name_input, role.id) {
                Ok((created, password)) => {
                    state.generated_password = Some(password);
                    state.generated_password_for = created.username.clone();
                    state.users.push(created);
                    state.users.sort_by(|a, b| a.username.cmp(&b.username));
                    state.username_input.clear();
                    state.display_name_input.clear();
                    state.role_input = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::DismissGeneratedPassword => {
            state.generated_password = None;
            state.generated_password_for.clear();
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(user) = state.users.iter().find(|u| u.id == Some(id)) {
                state.editing_id = Some(id);
                state.edit_role_input = state.role_choices.iter().find(|c| c.id == user.role_id).cloned();
                state.edit_is_active = user.is_active;
            }
        }
        Message::EditRoleSelected(choice) => state.edit_role_input = Some(choice),
        Message::EditActiveToggled(value) => state.edit_is_active = value,
        Message::SaveEdit => {
            state.message = None;
            state.error = None;
            let Some(id) = state.editing_id else { return };
            let Some(existing) = state.users.iter().find(|u| u.id == Some(id)).cloned() else {
                state.error = Some("that user no longer exists".into());
                state.editing_id = None;
                return;
            };
            let Some(role) = &state.edit_role_input else {
                state.error = Some("choose a role".into());
                return;
            };
            let updated = User { role_id: role.id, is_active: state.edit_is_active, ..existing };
            let service = AuthService::new(ctx.db.clone());
            match service.update_user(acting_user, &updated) {
                Ok(()) => {
                    if let Some(slot) = state.users.iter_mut().find(|u| u.id == Some(id)) {
                        *slot = updated;
                    }
                    state.editing_id = None;
                    state.message = Some("Saved changes.".into());
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.error = None;
        }
        Message::ResetPasswordForRow(id) => {
            state.message = None;
            state.error = None;
            let service = AuthService::new(ctx.db.clone());
            match service.reset_password(acting_user, id) {
                Ok(password) => {
                    let username = state.users.iter().find(|u| u.id == Some(id)).map(|u| u.username.clone()).unwrap_or_default();
                    state.generated_password = Some(password);
                    state.generated_password_for = username;
                    if let Some(slot) = state.users.iter_mut().find(|u| u.id == Some(id)) {
                        slot.must_change_password = true;
                        slot.is_reset_requested = false;
                    }
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, "Filters" / "Sort by" are
        // rendered by the shared toolbar component now (Section 6), and
        // there's no bulk action here -- deactivating is a deliberate
        // per-user decision, not something to batch. Wiring real
        // tab/filter/sort behavior is follow-up work, not part of this
        // pass.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked | Message::Noop => {}
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header = page_header::view(theme, "Users", "Everyone with access to this installation.", "Create User", Message::CreateUser);

    let toolbar = toolbar::view(
        theme,
        &["All Users"],
        0,
        Message::TabSelected,
        &state.search,
        "Search users",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "User list", "Every account, and whether a password reset has been requested.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&User> =
        state.users.iter().filter(|user| query.is_empty() || user.display_name.to_lowercase().contains(&query) || user.username.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("User", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Role", Length::FillPortion(2)),
        ColumnDef::new("Status", Length::FillPortion(1)),
        ColumnDef::new("Password Reset", Length::FillPortion(2)),
    ];

    let rows = filtered
        .iter()
        .map(|user| {
            Row::new(vec![
                Cell::person(user.display_name.clone(), user.username.clone()),
                Cell::text(role_name(&state.roles, user.role_id)),
                if user.is_active { Cell::status("Active", BadgeTone::Positive) } else { Cell::status("Inactive", BadgeTone::Neutral) },
                if user.is_reset_requested { Cell::status("Requested", BadgeTone::Warning) } else { Cell::text("--") },
            ])
            .with_primary_action("Reset Password")
        })
        .collect();

    let row_ids: Vec<i64> = filtered.iter().map(|u| u.id.unwrap_or_default()).collect();
    let edit_ids = row_ids.clone();
    let reset_ids = row_ids;

    let table = data_table::view(
        theme,
        columns,
        rows,
        None,
        |_index, _checked| Message::Noop,
        |_checked| Message::Noop,
        move |index| Message::EditRow(edit_ids[index]),
        move |index| Message::ResetPasswordForRow(reset_ids[index]),
    );

    let create_form = column![
        text("Create a user").size(typography::LABEL),
        text_input("Username", &state.username_input).style(inputs::text(theme)).on_input(Message::UsernameChanged).width(Length::Fill).padding(spacing::SM),
        text_input("Display name", &state.display_name_input).style(inputs::text(theme)).on_input(Message::DisplayNameChanged).width(Length::Fill).padding(spacing::SM),
        pick_list(state.role_choices.as_slice(), state.role_input.clone(), Message::RoleSelected).style(dropdown::style(theme)).placeholder("Role"),
        button(text("Create User")).style(button_style::primary(theme)).on_press(Message::CreateUser).padding(spacing::SM),
    ]
    .spacing(spacing::SM);

    let mut content = column![header, toolbar, heading, table].spacing(spacing::SPACE_20).padding(spacing::LG).width(Length::Fill);

    if let Some(password) = &state.generated_password {
        let notice = column![
            text(format!("Temporary password for {}", state.generated_password_for)).size(typography::LABEL),
            text(password.clone()).size(typography::SUBTITLE).color(theme.primary()),
            text("Shown once -- copy it now and hand it to the user directly. They'll be asked to set their own password on first login.").size(typography::BODY).color(theme.neutral_front()),
            button(text("Done")).style(button_style::secondary(theme)).on_press(Message::DismissGeneratedPassword).padding(spacing::SPACE_6),
        ]
        .spacing(spacing::SPACE_6)
        .padding(spacing::MD);
        content = content.push(container(notice).style(move |_iced_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme.positive_bg())),
            ..iced::widget::container::Style::default()
        }));
    }

    if let Some(id) = state.editing_id {
        let editing_user = state.users.iter().find(|u| u.id == Some(id));
        let heading_text = editing_user.map(|u| format!("Edit {}", u.display_name)).unwrap_or_else(|| "Edit user".to_string());
        let edit_panel = column![
            text(heading_text).size(typography::LABEL),
            pick_list(state.role_choices.as_slice(), state.edit_role_input.clone(), Message::EditRoleSelected).style(dropdown::style(theme)).placeholder("Role"),
            checkbox("Active", state.edit_is_active).style(checkbox_style::style(theme)).on_toggle(Message::EditActiveToggled),
            iced::widget::row![
                button(text("Save changes")).style(button_style::primary(theme)).on_press(Message::SaveEdit).padding(spacing::SM),
                button(text("Cancel")).style(button_style::secondary(theme)).on_press(Message::CancelEdit).padding(spacing::SM),
            ]
            .spacing(spacing::SM),
        ]
        .spacing(spacing::SM);
        content = content.push(edit_panel);
    }

    content = content.push(create_form);

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
    use crate::backend::core::auth::passwords::hash_password;
    use crate::backend::repositories::user_repository::{self, UserRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_admin_and_staff() -> (Ctx, User, RoleChoice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();

        let repo = UserRepository::new(db.clone());
        let mut admin_role = Role::new("Administrator");
        admin_role.permissions.push("users.manage".into());
        let admin_role = repo.create_role(&admin_role).unwrap();
        let staff_role = repo.create_role(&Role::new("Staff")).unwrap();
        let admin = repo.create_user(&User::new("admin", "Admin User", hash_password("correct horse battery staple").unwrap(), admin_role.id.unwrap())).unwrap();

        let ctx = Ctx { db, company_id: 1, current_user_id: admin.id, export_dir: PathBuf::from("."), uploads_dir: PathBuf::from(".") };
        (ctx, admin, RoleChoice { id: staff_role.id.unwrap(), label: staff_role.name })
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, admin, staff) = ctx_with_admin_and_staff();
        let mut state = State::default();

        update(&mut state, Message::UsernameChanged("carol".into()), &ctx, &admin);
        update(&mut state, Message::DisplayNameChanged("Carol Smith".into()), &ctx, &admin);
        update(&mut state, Message::RoleSelected(staff.clone()), &ctx, &admin);

        assert_eq!(state.username_input, "carol");
        assert_eq!(state.display_name_input, "Carol Smith");
        assert_eq!(state.role_input, Some(staff));
    }

    #[test]
    fn creating_a_user_adds_it_to_the_list_shows_the_password_once_and_clears_the_form() {
        let (ctx, admin, staff) = ctx_with_admin_and_staff();
        let mut state = State { username_input: "carol".into(), display_name_input: "Carol Smith".into(), role_input: Some(staff), ..load(&ctx) };

        update(&mut state, Message::CreateUser, &ctx, &admin);

        assert!(state.error.is_none());
        assert!(state.users.iter().any(|u| u.username == "carol" && u.must_change_password));
        assert_eq!(state.generated_password.as_ref().map(|p| p.len()), Some(16));
        assert_eq!(state.generated_password_for, "carol");
        assert!(state.username_input.is_empty());
        assert!(state.role_input.is_none());
    }

    #[test]
    fn creating_without_choosing_a_role_sets_an_error_and_creates_nobody() {
        let (ctx, admin, _staff) = ctx_with_admin_and_staff();
        let mut state = State { username_input: "carol".into(), display_name_input: "Carol Smith".into(), ..load(&ctx) };

        update(&mut state, Message::CreateUser, &ctx, &admin);

        assert!(state.error.is_some());
        assert!(!state.users.iter().any(|u| u.username == "carol"));
    }

    #[test]
    fn edit_row_prefills_the_role_and_active_status_from_the_existing_user() {
        let (ctx, admin, staff) = ctx_with_admin_and_staff();
        let mut state = State { username_input: "carol".into(), display_name_input: "Carol Smith".into(), role_input: Some(staff.clone()), ..load(&ctx) };
        update(&mut state, Message::CreateUser, &ctx, &admin);
        let id = state.users.iter().find(|u| u.username == "carol").unwrap().id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx, &admin);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.edit_role_input, Some(staff));
        assert!(state.edit_is_active);
    }

    #[test]
    fn saving_an_edit_changes_the_role_and_deactivates_the_user_in_place() {
        let (ctx, admin, staff) = ctx_with_admin_and_staff();
        let mut state = State { username_input: "carol".into(), display_name_input: "Carol Smith".into(), role_input: Some(staff), ..load(&ctx) };
        update(&mut state, Message::CreateUser, &ctx, &admin);
        let id = state.users.iter().find(|u| u.username == "carol").unwrap().id.unwrap();
        update(&mut state, Message::EditRow(id), &ctx, &admin);

        update(&mut state, Message::EditActiveToggled(false), &ctx, &admin);
        update(&mut state, Message::SaveEdit, &ctx, &admin);

        assert!(state.editing_id.is_none());
        let carol = state.users.iter().find(|u| u.id == Some(id)).unwrap();
        assert!(!carol.is_active);

        let reloaded = UserRepository::new(ctx.db.clone()).get_user(id).unwrap();
        assert!(!reloaded.is_active, "a deactivated user is preserved, not deleted");
    }

    #[test]
    fn deactivated_user_cannot_log_in_but_stays_fully_intact() {
        let (ctx, admin, staff) = ctx_with_admin_and_staff();
        let mut state = State { username_input: "carol".into(), display_name_input: "Carol Smith".into(), role_input: Some(staff), ..load(&ctx) };
        update(&mut state, Message::CreateUser, &ctx, &admin);
        let id = state.users.iter().find(|u| u.username == "carol").unwrap().id.unwrap();
        update(&mut state, Message::EditRow(id), &ctx, &admin);
        update(&mut state, Message::EditActiveToggled(false), &ctx, &admin);
        update(&mut state, Message::SaveEdit, &ctx, &admin);

        let service = AuthService::new(ctx.db.clone());
        let outcome = service.login_with_credentials("carol", "does-not-matter");

        assert!(outcome.is_err());
        let reloaded = UserRepository::new(ctx.db.clone()).get_user(id).unwrap();
        assert_eq!(reloaded.username, "carol");
        assert_eq!(reloaded.display_name, "Carol Smith");
    }

    #[test]
    fn resetting_a_password_generates_a_new_one_forces_a_change_and_clears_a_pending_request() {
        let (ctx, admin, staff) = ctx_with_admin_and_staff();
        let mut state = State { username_input: "carol".into(), display_name_input: "Carol Smith".into(), role_input: Some(staff), ..load(&ctx) };
        update(&mut state, Message::CreateUser, &ctx, &admin);
        let id = state.users.iter().find(|u| u.username == "carol").unwrap().id.unwrap();
        let first_password = state.generated_password.clone().unwrap();
        UserRepository::new(ctx.db.clone()).set_reset_requested(id, true).unwrap();
        state.users.iter_mut().find(|u| u.id == Some(id)).unwrap().is_reset_requested = true;

        update(&mut state, Message::ResetPasswordForRow(id), &ctx, &admin);

        assert!(state.error.is_none());
        let new_password = state.generated_password.clone().unwrap();
        assert_ne!(new_password, first_password);
        assert_eq!(state.generated_password_for, "carol");
        let carol = state.users.iter().find(|u| u.id == Some(id)).unwrap();
        assert!(carol.must_change_password);
        assert!(!carol.is_reset_requested);
    }
}
