//! A basic account switcher: pick another active user from a list
//! (rather than typing a username, like `login_view.rs` does) and
//! authenticate as them, without a full logout/login round trip.
//! Reuses `AuthService`'s login methods as-is -- switching is
//! authentication with a preselected username, nothing more.

use iced::widget::{button, column, container, pick_list, text, text_input};
use iced::Element;

use crate::backend::core::auth::authentication::AuthOutcome;
use crate::backend::core::users::User;
use crate::backend::services::auth_service::AuthService;
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::dropdown;
use crate::frontend::components::inputs;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    id: i64,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    available_users: Vec<(Choice, String)>, // (choice, username) -- username is what auth actually keys on
    selected_user: Option<Choice>,
    password: String,
    two_factor_code: String,
    pending_user: Option<User>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    UserSelected(Choice),
    PasswordChanged(String),
    TwoFactorCodeChanged(String),
    SubmitCredentials,
    SubmitTwoFactorCode,
}

pub enum Event {
    None,
    SwitchedTo(User),
}

pub fn load(ctx: &Ctx) -> State {
    let Some(current_user_id) = ctx.current_user_id else {
        return State::default();
    };
    let service = AuthService::new(ctx.db.clone());
    let available_users = service
        .list_other_active_users(current_user_id)
        .map(|users| {
            users
                .into_iter()
                .map(|u| {
                    (
                        Choice {
                            id: u.id.unwrap_or_default(),
                            label: u.display_name.clone(),
                        },
                        u.username,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    State {
        available_users,
        ..Default::default()
    }
}

fn username_for(state: &State, choice: &Choice) -> Option<String> {
    state
        .available_users
        .iter()
        .find(|(c, _)| c == choice)
        .map(|(_, username)| username.clone())
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) -> Event {
    match message {
        Message::UserSelected(choice) => {
            state.selected_user = Some(choice);
            state.error = None;
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
            let Some(selected) = state.selected_user.clone() else {
                state.error = Some("choose a user to switch to".into());
                return Event::None;
            };
            let Some(username) = username_for(state, &selected) else {
                state.error = Some("choose a user to switch to".into());
                return Event::None;
            };
            let service = AuthService::new(ctx.db.clone());
            match service.login_with_credentials(&username, &state.password) {
                Ok(AuthOutcome::Authenticated(user)) => Event::SwitchedTo(user),
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
                Ok(user) => Event::SwitchedTo(user),
                Err(err) => {
                    state.error = Some(err.to_string());
                    Event::None
                }
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    if state.available_users.is_empty() {
        return container(text("No other active users to switch to.").color(theme.primary()))
            .padding(spacing::SPACE_20)
            .into();
    }

    let choices: Vec<Choice> = state
        .available_users
        .iter()
        .map(|(c, _)| c.clone())
        .collect();

    let form: Element<'_, Message> = if state.pending_user.is_some() {
        column![
            text("Enter that user's two-factor authentication code"),
            text_input("6-digit code", &state.two_factor_code)
                .style(inputs::text(theme))
                .on_input(Message::TwoFactorCodeChanged)
                .padding(spacing::SM),
            button(text("Verify"))
                .style(button_style::primary(theme))
                .on_press(Message::SubmitTwoFactorCode)
                .padding(spacing::SM),
        ]
        .spacing(spacing::SPACE_12)
        .into()
    } else {
        column![
            text("Switch to"),
            pick_list(choices, state.selected_user.clone(), Message::UserSelected)
                .style(dropdown::style(theme))
                .placeholder("Choose a user"),
            text_input("Their password", &state.password)
                .style(inputs::text(theme))
                .on_input(Message::PasswordChanged)
                .secure(true)
                .padding(spacing::SM),
            button(text("Switch user"))
                .style(button_style::primary(theme))
                .on_press(Message::SubmitCredentials)
                .padding(spacing::SM),
        ]
        .spacing(spacing::SPACE_12)
        .into()
    };

    let mut content = column![text("Switch User").size(typography::title()), form]
        .spacing(spacing::MD)
        .padding(spacing::SPACE_20)
        .max_width(360);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(content).into()
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

    fn ctx_with_two_users() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();

        let repo = UserRepository::new(db.clone());
        let role = repo.create_role(&Role::new("Staff")).unwrap();
        let alice = repo
            .create_user(&User::new(
                "alice",
                "Alice Smith",
                hash_password("correct horse battery staple").unwrap(),
                role.id.unwrap(),
            ))
            .unwrap();
        repo.create_user(&User::new(
            "bob",
            "Bob Jones",
            hash_password("another good password").unwrap(),
            role.id.unwrap(),
        ))
        .unwrap();

        Ctx {
            db,
            company_id: 1,
            current_user_id: alice.id,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        }
    }

    #[test]
    fn load_lists_every_other_active_user_but_not_the_current_one() {
        let ctx = ctx_with_two_users();

        let state = load(&ctx);

        assert_eq!(state.available_users.len(), 1);
        assert_eq!(state.available_users[0].1, "bob");
    }

    #[test]
    fn submitting_without_choosing_a_user_sets_an_error() {
        let ctx = ctx_with_two_users();
        let mut state = State::default();

        let event = update(&mut state, Message::SubmitCredentials, &ctx);

        assert!(matches!(event, Event::None));
        assert_eq!(state.error.as_deref(), Some("choose a user to switch to"));
    }

    #[test]
    fn switching_with_the_correct_password_succeeds() {
        let ctx = ctx_with_two_users();
        let mut state = load(&ctx);
        let bob = state.available_users[0].0.clone();

        update(&mut state, Message::UserSelected(bob), &ctx);
        state.password = "another good password".into();
        let event = update(&mut state, Message::SubmitCredentials, &ctx);

        assert!(matches!(event, Event::SwitchedTo(user) if user.username == "bob"));
    }

    #[test]
    fn switching_with_the_wrong_password_is_rejected() {
        let ctx = ctx_with_two_users();
        let mut state = load(&ctx);
        let bob = state.available_users[0].0.clone();

        update(&mut state, Message::UserSelected(bob), &ctx);
        state.password = "not bob's password".into();
        let event = update(&mut state, Message::SubmitCredentials, &ctx);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_some());
    }

    #[test]
    fn switching_to_a_two_factor_account_requires_a_valid_code() {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(user_repository::MIGRATIONS).unwrap();
        let repo = UserRepository::new(db.clone());
        let role = repo.create_role(&Role::new("Staff")).unwrap();
        let alice = repo
            .create_user(&User::new(
                "alice",
                "Alice Smith",
                hash_password("correct horse battery staple").unwrap(),
                role.id.unwrap(),
            ))
            .unwrap();

        let secret = generate_secret();
        let mut bob = User::new(
            "bob",
            "Bob Jones",
            hash_password("another good password").unwrap(),
            role.id.unwrap(),
        );
        bob.two_factor_enabled = true;
        bob.two_factor_secret = Some(secret.clone());
        repo.create_user(&bob).unwrap();

        let ctx = Ctx {
            db,
            company_id: 1,
            current_user_id: alice.id,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        };
        let mut state = load(&ctx);
        let bob_choice = state.available_users[0].0.clone();

        update(&mut state, Message::UserSelected(bob_choice), &ctx);
        state.password = "another good password".into();
        let event = update(&mut state, Message::SubmitCredentials, &ctx);
        assert!(matches!(event, Event::None));
        assert!(state.pending_user.is_some());

        let code = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            totp_rs::Secret::Encoded(secret).to_bytes().unwrap(),
        )
        .unwrap()
        .generate_current()
        .unwrap();
        state.two_factor_code = code;
        let event = update(&mut state, Message::SubmitTwoFactorCode, &ctx);
        assert!(matches!(event, Event::SwitchedTo(user) if user.username == "bob"));
    }
}
