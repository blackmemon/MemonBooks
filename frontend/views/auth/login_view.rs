//! The login screen: username + password, then a TOTP code if the
//! account has 2FA enabled (Development Order step 4's two-step flow).

use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Length};

use crate::backend::core::auth::authentication::{authenticate, verify_two_factor, AuthOutcome};
use crate::backend::core::users::User;
use crate::backend::repositories::user_repository::UserRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    username: String,
    password: String,
    two_factor_code: String,
    pending_user: Option<User>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    UsernameChanged(String),
    PasswordChanged(String),
    TwoFactorCodeChanged(String),
    SubmitCredentials,
    SubmitTwoFactorCode,
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
            let user_repo = UserRepository::new(ctx.db.clone());
            let user = user_repo.find_user_by_username(&state.username).ok().flatten();
            match authenticate(user.as_ref(), &state.password) {
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
            match verify_two_factor(&user, &state.two_factor_code) {
                Ok(user) => Event::LoggedIn(user),
                Err(err) => {
                    state.error = Some(err.to_string());
                    Event::None
                }
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let title = text("MemonBooks").size(24).color(colors::WHITE_PRIMARY);

    let form: Element<'_, Message> = if state.pending_user.is_some() {
        column![
            text("Enter your two-factor authentication code"),
            text_input("6-digit code", &state.two_factor_code)
                .on_input(Message::TwoFactorCodeChanged)
                .padding(8),
            button(text("Verify")).on_press(Message::SubmitTwoFactorCode).padding(8),
        ]
        .spacing(12)
        .into()
    } else {
        column![
            text_input("Username", &state.username).on_input(Message::UsernameChanged).padding(8),
            text_input("Password", &state.password)
                .on_input(Message::PasswordChanged)
                .secure(true)
                .padding(8),
            button(text("Log in")).on_press(Message::SubmitCredentials).padding(8),
        ]
        .spacing(12)
        .into()
    };

    let error: Element<'_, Message> = match &state.error {
        Some(message) => text(message.clone()).color(colors::WHITE_NEGATIVE_FRONT).into(),
        None => text("").into(),
    };

    container(column![title, form, error].spacing(20).max_width(320))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
