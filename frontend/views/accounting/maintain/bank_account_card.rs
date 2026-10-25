//! The Bank Accounts card grid: one account as a horizontal card -- the
//! tinted tile on the left, the details (type, status, account number
//! with its show/hide eye, companies, credit and debit balances, Edit and
//! Delete) on the right, See details along the bottom. Edit the card's
//! layout here; its colors, sizes and shared widgets live in
//! `frontend/components/` and the field names in `BANK_FIELDS`
//! (`bank_of_accounts_view.rs`).

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};
use rust_decimal::Decimal;

use super::bank_of_accounts_view::{
    dash, status_label, type_tone, AccountSummary, Message, State, ARCHIVED_LABEL, BANK_FIELDS,
    F_ACCOUNT_NUMBER, F_CREDIT_BALANCE, F_DEBIT_BALANCE, F_PARENT_COMPANY, F_STATUS, F_SUB_COMPANY,
};
use crate::backend::banking::bank_accounts::BankAccount;
use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::labels::{self, Side};
use crate::frontend::components::row_snap;
use crate::frontend::components::table as table_style;
use crate::frontend::components::{icons, panels, spacing, typography};

const AVAILABLE_LABEL: &str = "Available Balance";
const SEE_DETAILS_LABEL: &str = "See details";
const SHOW_NUMBER_TIP: &str = "Show number";
const HIDE_NUMBER_TIP: &str = "Hide number";

/// The cards in `height` pixels, scrolling by whole rows, in a grid
/// `width` wide.
pub(super) fn card_grid<'a>(
    state: &'a State,
    theme: Theme,
    rows: &[&'a AccountSummary],
    (width, height): (f32, f32),
) -> Element<'a, Message> {
    let columns = spacing::card_columns(width);
    let stacked = spacing::card_stacks(spacing::card_width(width, columns));
    row_snap::view(
        &state.scroll,
        theme,
        rows.iter()
            .map(|summary| {
                let revealed = state.revealed.contains(&summary.id());
                account_card(theme, summary, revealed, stacked)
            })
            .collect(),
        columns,
        (height, spacing::compact_card_height(stacked)),
        Message::Scrolled,
    )
}

/// The type pill, plus an Archived pill for an inactive account.
pub(super) fn account_badges(
    theme: Theme,
    account: &BankAccount,
) -> Vec<Element<'static, Message>> {
    let kind = account.account_type;
    let mut pills = vec![badges::status_pill(theme, kind.as_str(), type_tone(kind))];
    if !account.is_active {
        pills.push(badges::status_pill(
            theme,
            ARCHIVED_LABEL,
            BadgeTone::Neutral,
        ));
    }
    pills
}

/// The tile of a card: bank name and type, the net balance, and the
/// masked account number (the full number shows in the details only).
fn card_tile<'a>(
    theme: Theme,
    summary: &'a AccountSummary,
    height: Length,
) -> Element<'a, Message> {
    let account = &summary.account;
    panels::card_face(
        theme,
        type_tone(account.account_type),
        panels::CardFace {
            caption: AVAILABLE_LABEL,
            figure: labels::currency(&account.currency_code, summary.balances.net()),
            line: labels::masked_number(&account.account_number, false),
            bottom_left: account.bank_name.as_str(),
            bottom_right: account.account_type.as_str(),
            height,
        },
    )
}

/// A value at the details' type size, cut off at its cell's edge.
fn cell_value<'a>(theme: Theme, value: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    container(labels::strong(theme, value, typography::label()).wrapping(Wrapping::None))
        .width(Length::Fill)
        .clip(true)
        .into()
}

/// The account number, masked or in full (wrapping rather than
/// overflowing), and its show/hide button.
fn number_cell<'a>(
    theme: Theme,
    account: &'a BankAccount,
    id: i64,
    revealed: bool,
) -> Element<'a, Message> {
    let (glyph, tip) = if revealed {
        (icons::EYE_OFF, HIDE_NUMBER_TIP)
    } else {
        (icons::EYE, SHOW_NUMBER_TIP)
    };
    labels::field(
        theme,
        BANK_FIELDS[F_ACCOUNT_NUMBER],
        row![
            labels::strong(
                theme,
                labels::masked_number(&account.account_number, revealed),
                typography::body()
            )
            .wrapping(Wrapping::Glyph)
            .width(Length::Fill),
            button_style::icon_tool_compact(theme, glyph, tip, Message::ToggleNumber(id)),
        ]
        .spacing(spacing::SPACE_4)
        .align_y(Alignment::Center),
    )
}

/// Two details side by side, sharing the line by `weights`.
fn pair_line<'a>(
    first: Element<'a, Message>,
    second: Element<'a, Message>,
    weights: (u16, u16),
) -> Element<'a, Message> {
    container(
        row![
            container(first).width(Length::FillPortion(weights.0)),
            container(second).width(Length::FillPortion(weights.1)),
        ]
        .spacing(spacing::SM),
    )
    .height(Length::Fill)
    .align_y(Alignment::Center)
    .into()
}

/// The details of a card: title, type and Edit/Delete, a divider, then
/// the other fields two to a line.
fn card_details<'a>(
    theme: Theme,
    summary: &'a AccountSummary,
    revealed: bool,
) -> Element<'a, Message> {
    let account = &summary.account;
    let id = summary.id();
    let kind = account.account_type;
    let money = |side: Side, amount: Decimal| {
        labels::amount(
            theme,
            labels::currency(&account.currency_code, amount),
            labels::side_tone(side),
            typography::label(),
        )
        .wrapping(Wrapping::None)
    };
    let status_tone = if account.is_active {
        BadgeTone::Positive
    } else {
        BadgeTone::Neutral
    };
    let field =
        |name: usize, value: Element<'a, Message>| labels::field(theme, BANK_FIELDS[name], value);
    let heading = row![
        labels::strong(
            theme,
            format!("{} Account", kind.as_str()),
            typography::panel_title()
        )
        .wrapping(Wrapping::None),
        badges::status_pill(theme, kind.as_str(), type_tone(kind)),
        Space::with_width(Length::Fill),
        button_style::icon_tool_compact(theme, icons::EDIT, "Edit", Message::EditRow(id)),
        button_style::icon_tool_compact(theme, icons::REMOVE, "Delete", Message::Remove(id)),
    ]
    .spacing(spacing::SPACE_6)
    .align_y(Alignment::Center);
    column![
        heading,
        table_style::divider_line(theme),
        pair_line(
            number_cell(theme, account, id, revealed),
            field(
                F_STATUS,
                badges::status_dot(theme, status_label(account.is_active), status_tone)
            ),
            (5, 2),
        ),
        pair_line(
            field(
                F_PARENT_COMPANY,
                cell_value(theme, dash(&account.parent_company))
            ),
            field(F_SUB_COMPANY, cell_value(theme, dash(&account.sub_company))),
            (1, 1),
        ),
        pair_line(
            field(
                F_CREDIT_BALANCE,
                money(Side::Credit, summary.balances.credit).into()
            ),
            field(
                F_DEBIT_BALANCE,
                money(Side::Debit, summary.balances.debit).into()
            ),
            (1, 1),
        ),
    ]
    .spacing(spacing::SPACE_4)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// One clickable account card: the tile and the details side by side
/// (the tile above when `stacked`), and a See details button along the
/// bottom.
fn account_card(
    theme: Theme,
    summary: &AccountSummary,
    revealed: bool,
    stacked: bool,
) -> Element<'_, Message> {
    let id = summary.id();
    let details = card_details(theme, summary, revealed);
    let body: Element<'_, Message> = if stacked {
        column![
            card_tile(theme, summary, Length::Fixed(spacing::tile_stack_height())),
            details
        ]
        .spacing(spacing::SPACE_8)
        .into()
    } else {
        row![
            container(card_tile(theme, summary, Length::Fill)).width(Length::FillPortion(2)),
            container(details).width(Length::FillPortion(3)),
        ]
        .spacing(spacing::SPACE_12)
        .into()
    };
    button(
        column![
            container(body).height(Length::Fill),
            button_style::wide(theme, SEE_DETAILS_LABEL, Message::OpenDetail(id)),
        ]
        .spacing(spacing::SPACE_8),
    )
    .padding(spacing::SPACE_12)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(button_style::tile(theme, BadgeTone::Neutral))
    .on_press(Message::OpenDetail(id))
    .into()
}

#[cfg(test)]
mod tests {
    use super::super::bank_of_accounts_view::tests::{
        ctx_with_a_gl_account, state_with, THEMES, WIDTHS,
    };
    use super::super::bank_of_accounts_view::update;
    use super::*;
    use crate::backend::banking::bank_accounts::BankAccountType;

    #[test]
    fn the_card_grid_builds_in_every_theme_and_width_with_any_number_of_cards() {
        for theme in THEMES {
            for width in WIDTHS {
                for count in [0, 1, 6, 7, 12] {
                    let mut state = state_with(count);
                    state.scroll.scroll(row_snap::Scroll::Lines(-1.0));
                    state.revealed.insert(1);
                    let rows: Vec<&AccountSummary> = state.accounts.iter().collect();
                    for height in [0.0, 300.0, 700.0, 1200.0] {
                        let _ = card_grid(&state, theme, &rows, (width, height));
                    }
                }
            }
        }
    }

    #[test]
    fn six_of_twelve_cards_show_at_once_on_wide_windows_in_whole_rows() {
        let state = state_with(12);
        let rows: Vec<&AccountSummary> = state.accounts.iter().collect();
        for (width, columns, shown_rows) in [(1366.0, 2, 3), (1920.0, 3, 2)] {
            assert_eq!(spacing::card_columns(width), columns, "{width}");
            let stacked = spacing::card_stacks(spacing::card_width(width, columns));
            assert!(!stacked, "{width}");
            let card = spacing::compact_card_height(stacked);
            let height = shown_rows as f32 * card + (shown_rows - 1) as f32 * spacing::ROW_GAP;
            let visible = row_snap::visible_rows(height, card, spacing::ROW_GAP);
            assert_eq!(visible, shown_rows, "{width}");
            let shown = row_snap::visible_cards(0, visible, columns, rows.len());
            assert_eq!(shown.len(), 6, "{width}");
            // One row per scroll step.
            assert_eq!(
                row_snap::visible_cards(1, visible, columns, rows.len()).start,
                columns
            );
            // A pixel short of the last row leaves only whole rows.
            assert_eq!(
                row_snap::visible_rows(height - 1.0, card, spacing::ROW_GAP),
                shown_rows - 1
            );
            for theme in THEMES {
                let _ = card_grid(&state, theme, &rows, (width, height));
            }
        }
    }

    #[test]
    fn the_columns_follow_the_card_minimum_width_and_a_narrow_column_stacks() {
        assert_eq!(spacing::card_columns(1920.0), 3);
        assert_eq!(spacing::card_columns(1366.0), 2);
        assert_eq!(spacing::card_columns(900.0), 1);
        assert_eq!(spacing::card_columns(640.0), 1);
        assert!(!spacing::card_stacks(spacing::card_width(640.0, 1)));
        assert!(spacing::card_stacks(spacing::card_width(400.0, 1)));
        assert!(spacing::compact_card_height(true) > spacing::compact_card_height(false));
    }

    #[test]
    fn toggling_a_number_reveals_and_hides_only_that_card() {
        let mut state = state_with(3);
        let (ctx, _) = ctx_with_a_gl_account();
        update(&mut state, Message::ToggleNumber(2), &ctx);
        assert!(state.revealed.contains(&2));
        assert!(!state.revealed.contains(&1));
        update(&mut state, Message::ToggleNumber(1), &ctx);
        update(&mut state, Message::ToggleNumber(2), &ctx);
        assert!(!state.revealed.contains(&2));
        assert!(state.revealed.contains(&1));
        let number = &state.accounts[0].account.account_number;
        assert_ne!(
            labels::masked_number(number, false),
            labels::masked_number(number, true)
        );
        assert_eq!(labels::masked_number(number, true), *number);
    }

    #[test]
    fn credit_and_debit_cards_get_different_tints_in_both_themes() {
        for theme in THEMES {
            let fill = |kind| panels::card_face_style(theme, type_tone(kind))(&iced::Theme::Light);
            assert_ne!(
                fill(BankAccountType::Credit).background,
                fill(BankAccountType::Debit).background
            );
        }
    }

    #[test]
    fn every_detail_of_an_account_is_built_into_its_card() {
        let state = state_with(2);
        for theme in THEMES {
            for summary in &state.accounts {
                for revealed in [false, true] {
                    for stacked in [false, true] {
                        let _ = account_card(theme, summary, revealed, stacked);
                    }
                    let _ = card_details(theme, summary, revealed);
                }
            }
        }
        // Every field the details show is drawn from BANK_FIELDS; the tile
        // and the heading carry the bank, type and net balance.
        let source = include_str!("bank_account_card.rs");
        let start = source.find("fn card_details").unwrap();
        let card = &source[start..source.find("fn account_card").unwrap()];
        for field in [
            "F_STATUS",
            "F_ACCOUNT_NUMBER",
            "F_PARENT_COMPANY",
            "F_SUB_COMPANY",
            "F_CREDIT_BALANCE",
            "F_DEBIT_BALANCE",
        ] {
            let source = if field == "F_ACCOUNT_NUMBER" {
                &source[source.find("fn number_cell").unwrap()..start]
            } else {
                card
            };
            assert!(source.contains(field), "{field}");
        }
        assert!(source.contains("balances.net()"));
        assert!(source.contains("bottom_left: account.bank_name"));
    }

    #[test]
    fn a_revealed_card_swaps_the_eye_and_the_number_in_its_details() {
        let state = state_with(1);
        let summary = &state.accounts[0];
        let number = &summary.account.account_number;
        assert_ne!(
            labels::masked_number(number, false),
            labels::masked_number(number, true)
        );
        for theme in THEMES {
            let _ = number_cell(theme, &summary.account, summary.id(), false);
            let _ = number_cell(theme, &summary.account, summary.id(), true);
        }
    }
}
