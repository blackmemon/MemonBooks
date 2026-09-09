//! Forced password change: the bootstrap-generated administrator
//! account (and anyone else with `User::must_change_password` set)
//! must pass through this screen before reaching the rest of the app --
//! `src/main.rs` gates on it ahead of normal route rendering, with no
//! way to navigate around it.

use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Length};

use crate::backend::core::users::User;
use crate::backend::services::auth_service::AuthService;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    current_password: String,
    new_password: String,
    confirm_password: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CurrentPasswordChanged(String),
    NewPasswordChanged(String),
    ConfirmPasswordChanged(String),
    Submit,
}

pub enum Event {
    None,
    Changed(User),
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx, current_user: &User) -> Event {
    match message {
        Message::CurrentPasswordChanged(value) => {
            state.current_password = value;
            Event::None
        }
        Message::NewPasswordChanged(value) => {
            state.new_password = value;
            Event::None
        }
        Message::ConfirmPasswordChanged(value) => {
            state.confirm_password = value;
            Event::None
        }
        Message::Submit => {
            if state.new_password != state.confirm_password {
                state.error = Some("new password and confirmation do not match".into());
                return Event::None;
            }
            let service = AuthService::new(ctx.db.clone());
            match service.change_password(current_user, &state.current_password, &state.new_password) {
                Ok(updated_user) => Event::Changed(updated_user),
                Err(err) => {
                    state.error = Some(err.to_string());
                    Event::None
                }
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let mut content = column![
        text("Change Your Password").size(typography::title()),
        text("You must set a new password before continuing.").color(theme.primary()),
        text("Current password"),
        text_input("Current password", &state.current_password)
            .style(inputs::text(theme))
            .on_input(Message::CurrentPasswordChanged)
            .secure(true)
            .padding(spacing::SM),
        text("New password"),
        text_input("New password", &state.new_password)
            .style(inputs::text(theme))
            .on_input(Message::NewPasswordChanged)
            .secure(true)
            .padding(spacing::SM),
        text("Confirm new password"),
        text_input("Confirm new password", &state.confirm_password)
            .style(inputs::text(theme))
            .on_input(Message::ConfirmPasswordChanged)
            .secure(true)
            .padding(spacing::SM),
        button(text("Change password")).style(button_style::primary(theme)).on_press(Message::Submit).padding(spacing::SM),
    ]
    .spacing(spacing::SPACE_12)
    .max_width(360);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::core::auth::passwords::hash_password;
    use crate::backend::core::roles::Role;
    use crate::backend::repositories::user_repository::{self, UserRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_user(password: &str, must_change_password: bool) -> (Ctx, User) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();

        let repo = UserRepository::new(db.clone());
        let role = repo.create_role(&Role::new("Administrator")).unwrap();
        let mut user = User::new("admin", "Administrator", hash_password(password).unwrap(), role.id.unwrap());
        user.must_change_password = must_change_password;
        let user = repo.create_user(&user).unwrap();

        let ctx = Ctx { db, company_id: 1, current_user_id: user.id, export_dir: PathBuf::from("."), uploads_dir: PathBuf::from(".") };
        (ctx, user)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, user) = ctx_with_user("correct horse battery staple", true);
        let mut state = State::default();

        update(&mut state, Message::CurrentPasswordChanged("old".into()), &ctx, &user);
        update(&mut state, Message::NewPasswordChanged("new-password-123".into()), &ctx, &user);
        update(&mut state, Message::ConfirmPasswordChanged("new-password-123".into()), &ctx, &user);

        assert_eq!(state.current_password, "old");
        assert_eq!(state.new_password, "new-password-123");
        assert_eq!(state.confirm_password, "new-password-123");
    }

    #[test]
    fn mismatched_confirmation_sets_an_error_without_calling_the_service() {
        let (ctx, user) = ctx_with_user("correct horse battery staple", true);
        let mut state = State {
            current_password: "correct horse battery staple".into(),
            new_password: "new-password-123".into(),
            confirm_password: "something-else-entirely".into(),
            ..Default::default()
        };

        let event = update(&mut state, Message::Submit, &ctx, &user);

        assert!(matches!(event, Event::None));
        assert_eq!(state.error.as_deref(), Some("new password and confirmation do not match"));
    }

    #[test]
    fn wrong_current_password_is_rejected() {
        let (ctx, user) = ctx_with_user("correct horse battery staple", true);
        let mut state = State {
            current_password: "totally wrong".into(),
            new_password: "new-password-123".into(),
            confirm_password: "new-password-123".into(),
            ..Default::default()
        };

        let event = update(&mut state, Message::Submit, &ctx, &user);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_some());
    }

    #[test]
    fn a_new_password_that_is_too_short_is_rejected() {
        let (ctx, user) = ctx_with_user("correct horse battery staple", true);
        let mut state = State {
            current_password: "correct horse battery staple".into(),
            new_password: "short".into(),
            confirm_password: "short".into(),
            ..Default::default()
        };

        let event = update(&mut state, Message::Submit, &ctx, &user);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_some());
    }

    #[test]
    fn a_valid_change_clears_must_change_password_and_the_new_password_verifies() {
        let (ctx, user) = ctx_with_user("correct horse battery staple", true);
        let mut state = State {
            current_password: "correct horse battery staple".into(),
            new_password: "new-password-123".into(),
            confirm_password: "new-password-123".into(),
            ..Default::default()
        };

        let event = update(&mut state, Message::Submit, &ctx, &user);

        let Event::Changed(updated_user) = event else { panic!("expected Event::Changed") };
        assert!(!updated_user.must_change_password);

        let refetched = UserRepository::new(ctx.db.clone()).get_user(user.id.unwrap()).unwrap();
        assert!(!refetched.must_change_password);
        assert!(crate::backend::core::auth::passwords::verify_password("new-password-123", &refetched.password_hash).unwrap());
    }
}
