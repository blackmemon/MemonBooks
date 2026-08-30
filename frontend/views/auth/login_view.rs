//! The login screen: username + password, then a TOTP code if the
//! account has 2FA enabled (Development Order step 4's two-step flow).

use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Length};

use crate::backend::core::auth::authentication::AuthOutcome;
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
    username: String,
    password: String,
    two_factor_code: String,
    pending_user: Option<User>,
    /// Shows the "Forgot Password" screen instead of the credentials
    /// form when set. Doesn't reset anything itself -- only an
    /// administrator can do that from the Users screen; this just lets
    /// the account owner flag their account so an admin knows to.
    forgot_password: bool,
    reset_request_username: String,
    reset_request_submitted: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    UsernameChanged(String),
    PasswordChanged(String),
    TwoFactorCodeChanged(String),
    SubmitCredentials,
    SubmitTwoFactorCode,
    ForgotPasswordClicked,
    BackToLoginClicked,
    ResetRequestUsernameChanged(String),
    SubmitResetRequest,
}

pub enum Event {
    None,
    LoggedIn(User),
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) -> Event {
    match message {
        Message::UsernameChanged(value) => {
            state.username = value;
            Event::None
        }
        Message::PasswordChanged(value) => {
            state.password = value;
            Event::None
        }
        Message::TwoFactorCodeChanged(value) => {
            state.two_factor_code = value;
            Event::None
        }
        Message::SubmitCredentials => {
            let service = AuthService::new(ctx.db.clone());
            match service.login_with_credentials(&state.username, &state.password) {
                Ok(AuthOutcome::Authenticated(user)) => Event::LoggedIn(user),
                Ok(AuthOutcome::TwoFactorRequired(user)) => {
                    state.pending_user = Some(user);
                    state.error = None;
                    Event::None
                }
                Err(err) => {
                    state.error = Some(err.to_string());
                    Event::None
                }
            }
        }
        Message::SubmitTwoFactorCode => {
            let Some(user) = state.pending_user.clone() else {
                return Event::None;
            };
            let service = AuthService::new(ctx.db.clone());
            match service.login_with_two_factor_code(&user, &state.two_factor_code) {
                Ok(user) => Event::LoggedIn(user),
                Err(err) => {
                    state.error = Some(err.to_string());
                    Event::None
                }
            }
        }
        Message::ForgotPasswordClicked => {
            state.forgot_password = true;
            state.reset_request_username = state.username.clone();
            state.reset_request_submitted = false;
            state.error = None;
            Event::None
        }
        Message::BackToLoginClicked => {
            state.forgot_password = false;
            state.reset_request_submitted = false;
            Event::None
        }
        Message::ResetRequestUsernameChanged(value) => {
            state.reset_request_username = value;
            Event::None
        }
        Message::SubmitResetRequest => {
            let service = AuthService::new(ctx.db.clone());
            // Always shows the same confirmation regardless of outcome
            // -- see `AuthService::request_password_reset`'s doc
            // comment for why this can't reveal whether the username
            // exists.
            let _ = service.request_password_reset(&state.reset_request_username);
            state.reset_request_submitted = true;
            Event::None
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let title = text("MemonBooks").size(typography::TITLE).color(theme.primary());

    let form: Element<'_, Message> = if state.forgot_password {
        if state.reset_request_submitted {
            column![
                text("If an account with that username exists, an administrator has been notified and can reset your password for you.").size(typography::LABEL),
                button(text("Back to login")).style(button_style::secondary(theme)).on_press(Message::BackToLoginClicked).padding(spacing::SM),
            ]
            .spacing(spacing::SPACE_12)
            .into()
        } else {
            column![
                text("Only an administrator can reset your password. Enter your username below and we'll flag your account so an administrator knows to help.").size(typography::LABEL),
                text_input("Username", &state.reset_request_username).style(inputs::text(theme)).on_input(Message::ResetRequestUsernameChanged).padding(spacing::SM),
                button(text("Request Reset")).style(button_style::primary(theme)).on_press(Message::SubmitResetRequest).padding(spacing::SM),
                button(text("Back to login")).style(button_style::secondary(theme)).on_press(Message::BackToLoginClicked).padding(spacing::SM),
            ]
            .spacing(spacing::SPACE_12)
            .into()
        }
    } else if state.pending_user.is_some() {
        column![
            text("Enter your two-factor authentication code"),
            text_input("6-digit code", &state.two_factor_code)
                .style(inputs::text(theme))
                .on_input(Message::TwoFactorCodeChanged)
                .padding(spacing::SM),
            button(text("Verify")).style(button_style::primary(theme)).on_press(Message::SubmitTwoFactorCode).padding(spacing::SM),
        ]
        .spacing(spacing::SPACE_12)
        .into()
    } else {
        column![
            text_input("Username", &state.username).style(inputs::text(theme)).on_input(Message::UsernameChanged).padding(spacing::SM),
            text_input("Password", &state.password)
                .style(inputs::text(theme))
                .on_input(Message::PasswordChanged)
                .secure(true)
                .padding(spacing::SM),
            button(text("Log in")).style(button_style::primary(theme)).on_press(Message::SubmitCredentials).padding(spacing::SM),
            button(text("Forgot Password?")).style(button_style::secondary(theme)).on_press(Message::ForgotPasswordClicked).padding(spacing::SM),
        ]
        .spacing(spacing::SPACE_12)
        .into()
    };

    let error: Element<'_, Message> = match &state.error {
        Some(message) => text(message.clone()).color(theme.negative_front()).into(),
        None => text("").into(),
    };

    container(column![title, form, error].spacing(spacing::SPACE_20).max_width(320))
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
    use crate::backend::core::auth::two_factor::generate_secret;
    use crate::backend::core::roles::Role;
    use crate::backend::repositories::user_repository::{self, UserRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_user(username: &str, password: &str, two_factor_secret: Option<String>) -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();

        let repo = UserRepository::new(db.clone());
        let role = repo.create_role(&Role::new("Staff")).unwrap();
        let mut user = User::new(username, "Test User", hash_password(password).unwrap(), role.id.unwrap());
        if let Some(secret) = two_factor_secret {
            user.two_factor_enabled = true;
            user.two_factor_secret = Some(secret);
        }
        repo.create_user(&user).unwrap();

        Ctx { db, company_id: 1, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let mut state = State::default();
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);

        update(&mut state, Message::UsernameChanged("alice".into()), &ctx);
        update(&mut state, Message::PasswordChanged("secret".into()), &ctx);
        update(&mut state, Message::TwoFactorCodeChanged("123456".into()), &ctx);

        assert_eq!(state.username, "alice");
        assert_eq!(state.password, "secret");
        assert_eq!(state.two_factor_code, "123456");
    }

    #[test]
    fn correct_credentials_without_two_factor_logs_in_directly() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { username: "alice".into(), password: "correct horse battery staple".into(), ..Default::default() };

        let event = update(&mut state, Message::SubmitCredentials, &ctx);

        assert!(matches!(event, Event::LoggedIn(user) if user.username == "alice"));
        assert!(state.error.is_none());
    }

    #[test]
    fn wrong_password_sets_a_generic_error_and_does_not_log_in() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { username: "alice".into(), password: "wrong password".into(), ..Default::default() };

        let event = update(&mut state, Message::SubmitCredentials, &ctx);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_some());
    }

    #[test]
    fn unknown_username_sets_the_same_generic_error_as_a_wrong_password() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { username: "someone-else".into(), password: "whatever".into(), ..Default::default() };

        update(&mut state, Message::SubmitCredentials, &ctx);

        assert_eq!(state.error.as_deref(), Some("validation error: invalid username or password"));
    }

    #[test]
    fn correct_credentials_with_two_factor_enabled_requires_a_code_before_logging_in() {
        let secret = generate_secret();
        let ctx = ctx_with_user("alice", "correct horse battery staple", Some(secret.clone()));
        let mut state = State { username: "alice".into(), password: "correct horse battery staple".into(), ..Default::default() };

        let event = update(&mut state, Message::SubmitCredentials, &ctx);
        assert!(matches!(event, Event::None));
        assert!(state.pending_user.is_some());

        let code = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, totp_rs::Secret::Encoded(secret).to_bytes().unwrap())
            .unwrap()
            .generate_current()
            .unwrap();
        state.two_factor_code = code;

        let event = update(&mut state, Message::SubmitTwoFactorCode, &ctx);
        assert!(matches!(event, Event::LoggedIn(user) if user.username == "alice"));
    }

    #[test]
    fn wrong_two_factor_code_sets_an_error_and_does_not_log_in() {
        let secret = generate_secret();
        let ctx = ctx_with_user("alice", "correct horse battery staple", Some(secret));
        let mut state = State { username: "alice".into(), password: "correct horse battery staple".into(), ..Default::default() };

        update(&mut state, Message::SubmitCredentials, &ctx);
        assert!(state.pending_user.is_some());

        state.two_factor_code = "000000".into();
        let event = update(&mut state, Message::SubmitTwoFactorCode, &ctx);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_some());
    }

    #[test]
    fn submitting_a_two_factor_code_with_no_pending_user_is_a_no_op() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State::default();

        let event = update(&mut state, Message::SubmitTwoFactorCode, &ctx);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_none());
    }

    #[test]
    fn forgot_password_clicked_switches_to_the_reset_request_screen_prefilled_with_the_typed_username() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { username: "alice".into(), ..Default::default() };

        update(&mut state, Message::ForgotPasswordClicked, &ctx);

        assert!(state.forgot_password);
        assert_eq!(state.reset_request_username, "alice");
        assert!(!state.reset_request_submitted);
    }

    #[test]
    fn back_to_login_clicked_returns_to_the_credentials_screen() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { forgot_password: true, reset_request_submitted: true, ..Default::default() };

        update(&mut state, Message::BackToLoginClicked, &ctx);

        assert!(!state.forgot_password);
        assert!(!state.reset_request_submitted);
    }

    #[test]
    fn submitting_a_reset_request_for_a_real_username_flags_the_account() {
        use crate::backend::repositories::user_repository::UserRepository;

        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { forgot_password: true, reset_request_username: "alice".into(), ..Default::default() };

        update(&mut state, Message::SubmitResetRequest, &ctx);

        assert!(state.reset_request_submitted);
        let repo = UserRepository::new(ctx.db.clone());
        let user = repo.find_user_by_username("alice").unwrap().unwrap();
        assert!(user.is_reset_requested);
    }

    #[test]
    fn submitting_a_reset_request_for_an_unknown_username_still_shows_the_same_confirmation() {
        let ctx = ctx_with_user("alice", "correct horse battery staple", None);
        let mut state = State { forgot_password: true, reset_request_username: "nobody-by-this-name".into(), ..Default::default() };

        update(&mut state, Message::SubmitResetRequest, &ctx);

        // No error, no distinguishable outcome -- proves this can't be
        // used to probe which usernames exist.
        assert!(state.reset_request_submitted);
        assert!(state.error.is_none());
    }
}
