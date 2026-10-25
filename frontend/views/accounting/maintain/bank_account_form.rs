//! The Bank Accounts add/edit side panel: text inputs, the bank, company
//! and status dropdowns, the type and ledger-account pickers, the error
//! line and Save/Cancel. Edit the form's fields here; validation and
//! saving are in `bank_of_accounts_view.rs`, shared widgets in
//! `frontend/components/`.

use iced::widget::{column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use super::bank_of_accounts_view::{
    status_label, Message, State, ACTIVE_LABEL, ADD_LABEL, ARCHIVED_LABEL, BANK_FIELDS,
    F_ACCOUNT_NUMBER, F_BANK_NAME, F_LEDGER_ACCOUNT, F_OPENING_BALANCE, F_PARENT_COMPANY, F_STATUS,
    F_SUB_COMPANY, F_TYPE, NONE_LABEL,
};
use crate::backend::banking::bank_accounts::{BANK_ACCOUNT_TYPES, PAKISTANI_BANKS};
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{
    dropdown, forms, labels, panels, scrollbar, spacing, typography,
};

const STATUS_OPTIONS: [&str; 2] = [ACTIVE_LABEL, ARCHIVED_LABEL];

/// The add/edit panel, with a Back button when it replaces the page.
pub(super) fn form_panel(
    state: &State,
    theme: Theme,
    class: spacing::LayoutClass,
    width: f32,
) -> Element<'_, Message> {
    let mut side = column![]
        .spacing(spacing::SPACE_20)
        .width(Length::Fixed(width))
        .height(Length::Fill);
    if class == spacing::LayoutClass::Narrow {
        side = side.push(button_style::back(theme, Message::CancelEdit));
    }
    side.push(account_form(state, theme)).into()
}

/// A labelled text input for one of `BANK_FIELDS`.
fn input_field<'a>(
    theme: Theme,
    field: usize,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    let label = BANK_FIELDS[field];
    forms::text_field(theme, label, text_input(label, value).on_input(on_input))
}

/// A labelled dropdown for one of `BANK_FIELDS` over text options.
fn select_field<'a>(
    theme: Theme,
    field: usize,
    options: Vec<String>,
    selected: Option<String>,
    on_select: fn(String) -> Message,
) -> Element<'a, Message> {
    forms::field_row(
        theme,
        BANK_FIELDS[field],
        dropdown::form_select(theme, options, selected, on_select)
            .placeholder(BANK_FIELDS[field])
            .into(),
    )
}

fn non_blank(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_string())
}

/// The bank picker: the known banks plus the account's own bank.
fn bank_select<'a>(theme: Theme, current: &str) -> Element<'a, Message> {
    select_field(
        theme,
        F_BANK_NAME,
        dropdown::options_with(PAKISTANI_BANKS, current),
        non_blank(current),
        Message::BankNameChanged,
    )
}

/// A company picker (`field` is Parent or Sub Company) over every
/// company plus the saved value; `with_none` adds an empty choice.
fn company_select<'a>(
    theme: Theme,
    field: usize,
    companies: &'a [String],
    current: &str,
    with_none: bool,
) -> Element<'a, Message> {
    let mut options = Vec::new();
    if with_none {
        options.push(NONE_LABEL.to_string());
    }
    options.extend(dropdown::options_with(
        companies.iter().map(String::as_str),
        current,
    ));
    let selected = non_blank(current).or_else(|| with_none.then(|| NONE_LABEL.to_string()));
    let on_select = if with_none {
        Message::SubCompanyChanged
    } else {
        Message::ParentCompanyChanged
    };
    select_field(theme, field, options, selected, on_select)
}

/// The Active / Archived picker.
fn status_select<'a>(theme: Theme, active: bool) -> Element<'a, Message> {
    forms::field_row(
        theme,
        BANK_FIELDS[F_STATUS],
        dropdown::form_select(
            theme,
            &STATUS_OPTIONS[..],
            Some(status_label(active)),
            |label| Message::StatusChanged(label == ACTIVE_LABEL),
        )
        .into(),
    )
}

fn account_form(state: &State, theme: Theme) -> Element<'_, Message> {
    let form = &state.form;
    let fields = vec![
        bank_select(theme, &form.bank_name),
        input_field(
            theme,
            F_ACCOUNT_NUMBER,
            &form.account_number,
            Message::AccountNumberChanged,
        ),
        company_select(
            theme,
            F_PARENT_COMPANY,
            &state.companies,
            &form.parent_company,
            false,
        ),
        company_select(
            theme,
            F_SUB_COMPANY,
            &state.companies,
            &form.sub_company,
            true,
        ),
        forms::field_row(
            theme,
            BANK_FIELDS[F_TYPE],
            dropdown::form_select(
                theme,
                &BANK_ACCOUNT_TYPES[..],
                Some(form.account_type),
                Message::TypeChanged,
            )
            .into(),
        ),
        input_field(
            theme,
            F_OPENING_BALANCE,
            &form.opening_balance,
            Message::OpeningBalanceChanged,
        ),
        status_select(theme, form.active),
        forms::field_row(
            theme,
            BANK_FIELDS[F_LEDGER_ACCOUNT],
            dropdown::form_select(
                theme,
                state.gl_accounts.as_slice(),
                form.gl_account.clone(),
                Message::GlAccountSelected,
            )
            .placeholder(BANK_FIELDS[F_LEDGER_ACCOUNT])
            .into(),
        ),
    ];
    let title = if state.editing_id.is_some() {
        "Edit Account"
    } else {
        ADD_LABEL
    };
    let mut body = column![
        labels::strong(theme, title, typography::panel_title()),
        scrollable(container(forms::field_grid(fields, 1)).padding(scrollbar::gutter()))
            .direction(scrollbar::vertical())
            .height(Length::Fill),
    ]
    .spacing(spacing::MD);
    if let Some(error) = &state.form_error {
        body = body.push(text(error.clone()).color(theme.negative_front()));
    }
    body = body.push(
        row![
            button_style::bold("Save", [spacing::SM, spacing::MD])
                .style(button_style::mode(theme, true))
                .on_press(Message::Create),
            button_style::bold("Cancel", [spacing::SM, spacing::MD])
                .style(button_style::outlined(theme, false))
                .on_press(Message::CancelEdit),
        ]
        .spacing(spacing::SM),
    );
    panels::side(theme, body)
}

#[cfg(test)]
mod tests {
    use super::super::bank_of_accounts_view::tests::{
        build_everywhere, ctx_with_a_gl_account, ledger, state_with, THEMES,
    };
    use super::super::bank_of_accounts_view::{form_account, open_form, update, Choice, Detail};
    use super::*;

    #[test]
    fn the_form_dropdowns_always_offer_the_saved_value() {
        let banks = dropdown::options_with(PAKISTANI_BANKS, "Habib Bank Limited");
        assert_eq!(banks.len(), PAKISTANI_BANKS.len());
        let custom = dropdown::options_with(PAKISTANI_BANKS, "Local Co-operative Bank");
        assert_eq!(
            custom.last().map(String::as_str),
            Some("Local Co-operative Bank")
        );
        let companies = ["Memon Group".to_string(), "Memon Textiles".to_string()];
        let names = || companies.iter().map(String::as_str);
        assert!(dropdown::options_with(names(), "Retired Co").contains(&"Retired Co".to_string()));
        for theme in THEMES {
            let _ = bank_select(theme, "Local Co-operative Bank");
            let _ = company_select(theme, F_PARENT_COMPANY, &companies, "Retired Co", false);
            let _ = company_select(theme, F_SUB_COMPANY, &companies, "", true);
            let _ = status_select(theme, true);
            let _ = status_select(theme, false);
        }
    }

    #[test]
    fn the_status_and_sub_company_pickers_update_the_form() {
        let mut state = state_with(1);
        let (ctx, _) = ctx_with_a_gl_account();
        open_form(&mut state, Some(1));
        assert!(state.form.active);
        update(&mut state, Message::StatusChanged(false), &ctx);
        assert!(!state.form.active);
        update(
            &mut state,
            Message::SubCompanyChanged("Memon Textiles".into()),
            &ctx,
        );
        assert_eq!(state.form.sub_company, "Memon Textiles");
        update(
            &mut state,
            Message::SubCompanyChanged(NONE_LABEL.into()),
            &ctx,
        );
        assert!(state.form.sub_company.is_empty());
        state.form.gl_account = Some(Choice {
            id: 10,
            label: "1000 - Bank".into(),
        });
        assert!(!form_account(&state).expect("a valid form").is_active);
    }

    #[test]
    fn the_add_form_builds_at_every_width_and_theme() {
        let mut state = state_with(3);
        state.form_open = true;
        build_everywhere(&state);
        state.form_error = Some("choose a GL account".into());
        build_everywhere(&state);
    }

    #[test]
    fn the_edit_form_builds_at_every_width_and_theme() {
        let mut state = state_with(3);
        state.gl_accounts = vec![Choice {
            id: 10,
            label: "1010 - Bank".into(),
        }];
        let summary = state.accounts[1].clone();
        let id = summary.id();
        state.form_open = true;
        state.editing_id = Some(id);
        state.form.bank_name = summary.account.bank_name.clone();
        state.form.gl_account = state.gl_accounts.first().cloned();
        build_everywhere(&state);
        // The panel also opens over the detail page.
        state.detail = Some(Detail {
            ledger: ledger(3),
            ..Detail::new(id)
        });
        build_everywhere(&state);
    }
}
