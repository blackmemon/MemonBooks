//! Generic card/panel container style (Section 6): white background,
//! rounded corners, a hairline border and a contained shadow. Used by the
//! data table card, the sidebar's user profile card, and any future
//! panel that wants the same "floating card" treatment.

use iced::widget::text::IntoFragment;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, stack, text, Space, Text};
use iced::{Alignment, Background, Element, Length, Theme as IcedTheme};

use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{
    borders, button as button_style, fonts, icons, labels, scrollbar, shadows, spacing, states,
    table, typography,
};

/// The one outer card of a list page (Vendors, Bank Accounts,
/// Warehouses): `header` (title, toolbar), the `body` filling the middle,
/// and, when given, a divider and the `footer` pinned under it -- all
/// inside a single rounded, bordered card that fills the page area.
pub fn page_card<'a, M: 'a>(
    theme: Theme,
    header: Element<'a, M>,
    body: Element<'a, M>,
    footer: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let mut card = column![header, body].height(Length::Fill);
    if let Some(footer) = footer {
        card = card.push(table::divider_line(theme)).push(footer);
    }
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::SPACE_1)
        .style(table::card(theme, states::RADIUS_LG))
        .into()
}

/// Padded flat card spanning the width -- a block inside a panel.
pub fn padded<'a, M: 'a>(theme: Theme, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(spacing::MD)
        .width(Length::Fill)
        .style(flat_card(theme, states::RADIUS_MD))
        .into()
}

/// [`padded`] card with a bold primary title above its content.
pub fn titled_card<'a, M: 'a>(
    theme: Theme,
    title: impl IntoFragment<'a>,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    padded(
        theme,
        column![
            text(title)
                .size(typography::emphasis())
                .font(fonts::BOLD)
                .color(theme.primary()),
            content.into(),
        ]
        .spacing(spacing::SPACE_10),
    )
}

/// A side panel's own card: padded, filling the space it is given.
pub fn side<'a, M: 'a>(theme: Theme, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(spacing::MD)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(flat_card(theme, states::RADIUS_LG))
        .into()
}

pub fn card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.page_bg())),
        border: borders::card(theme, radius),
        shadow: shadows::card(theme),
        ..container::Style::default()
    }
}

/// Same card, no shadow -- for panels nested inside an already-shadowed
/// ancestor (e.g. the profile card at the bottom of the sidebar).
pub fn flat_card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.page_bg())),
        border: borders::card(theme, radius),
        ..container::Style::default()
    }
}

/// What a [`card_face`] shows, top to bottom.
pub struct CardFace<'a> {
    /// A muted caption over the big figure.
    pub caption: &'a str,
    pub figure: String,
    /// The masked line near the bottom.
    pub line: String,
    pub bottom_left: &'a str,
    pub bottom_right: &'a str,
    /// Fixed when the tile sits above something, `Fill` beside it.
    pub height: Length,
}

/// The tinted fill of a [`card_face`]: `tone`'s own background, so two
/// tones always look different and stay readable in both themes.
pub fn card_face_style(theme: Theme, tone: BadgeTone) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(badges::tone_colors(theme, tone).0)),
        border: borders::card(theme, states::RADIUS_LG),
        ..container::Style::default()
    }
}

/// A payment-card style tile: a two-circle logo mark and a chip on top,
/// a caption over a big figure in `tone`'s color, then a masked line and
/// two corner texts at the bottom. It fills the height it is given.
pub fn card_face<'a, M: 'a>(theme: Theme, tone: BadgeTone, face: CardFace<'a>) -> Element<'a, M> {
    let circle = || {
        icons::icon(icons::LOGO_CIRCLE)
            .size(typography::emphasis())
            .color(badges::tone_colors(theme, tone).1)
    };
    let logo = stack![
        circle(),
        container(circle()).padding(iced::Padding {
            left: spacing::SPACE_8,
            ..iced::Padding::ZERO
        }),
    ];
    let top = row![
        logo,
        Space::with_width(Length::Fill),
        icons::icon(icons::CHIP)
            .size(typography::subtitle())
            .color(theme.neutral_front()),
    ]
    .align_y(Alignment::Center);
    let clipped = |value: Text<'a>| {
        container(value.wrapping(Wrapping::None))
            .width(Length::Fill)
            .clip(true)
    };
    let bottom = row![
        clipped(labels::strong(
            theme,
            face.bottom_left,
            typography::caption()
        )),
        labels::strong(theme, face.bottom_right, typography::body()).wrapping(Wrapping::None),
    ]
    .spacing(spacing::SPACE_8);
    container(
        column![
            top,
            Space::with_height(Length::Fill),
            labels::muted(theme, face.caption, typography::caption()),
            clipped(labels::amount(
                theme,
                face.figure,
                tone,
                typography::subtitle()
            )),
            Space::with_height(Length::Fill),
            clipped(labels::muted(theme, face.line, typography::label())),
            bottom,
        ]
        .spacing(spacing::SPACE_2),
    )
    .padding(spacing::SPACE_10)
    .width(Length::Fill)
    .height(face.height)
    .style(card_face_style(theme, tone))
    .into()
}

/// A centered "nothing here yet" block: a muted icon, a bold message, an
/// optional hint and an optional action button under them.
pub fn empty_state<'a, M: 'a>(
    theme: Theme,
    glyph: &'static str,
    message: impl IntoFragment<'a>,
    hint: Option<&'a str>,
    action: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let mut content = column![
        crate::frontend::components::icons::icon(glyph)
            .size(typography::title())
            .color(theme.neutral_front()),
        text(message)
            .size(typography::panel_title())
            .font(fonts::BOLD)
            .color(theme.neutral_font()),
    ]
    .spacing(spacing::SPACE_10)
    .align_x(iced::Alignment::Center);
    if let Some(hint) = hint {
        content = content.push(
            text(hint)
                .size(typography::body())
                .color(theme.neutral_front()),
        );
    }
    if let Some(action) = action {
        content = content.push(action);
    }
    container(content)
        .padding(spacing::SPACE_40)
        .center_x(Length::Fill)
        .style(flat_card(theme, states::RADIUS_LG))
        .into()
}

/// A thin rounded bar filled to `fraction` (0..=1) in `tone`'s text
/// color over the neutral track.
pub fn progress_bar<'a, M: 'a>(
    theme: Theme,
    fraction: f32,
    tone: crate::frontend::components::badges::BadgeTone,
) -> Element<'a, M> {
    iced::widget::progress_bar(0.0..=1.0, fraction.clamp(0.0, 1.0))
        .height(Length::Fixed(spacing::PROGRESS_BAR_HEIGHT))
        .style(
            move |_iced_theme: &IcedTheme| iced::widget::progress_bar::Style {
                background: Background::Color(theme.neutral_bg()),
                bar: Background::Color(match tone {
                    crate::frontend::components::badges::BadgeTone::Neutral => theme.primary(),
                    tone => crate::frontend::components::badges::tone_colors(theme, tone).1,
                }),
                border: iced::Border {
                    radius: states::RADIUS_PILL.into(),
                    ..iced::Border::default()
                },
            },
        )
        .into()
}

/// A titled section whose header (title, chevron) toggles `body` open or
/// closed, with a thin rule under the header. The caller owns `open`.
pub fn collapsible_section<'a, M: Clone + 'a>(
    theme: Theme,
    title: impl IntoFragment<'a>,
    open: bool,
    on_toggle: M,
    body: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    let header = button(
        row![
            labels::strong(theme, title, typography::emphasis()).width(Length::Fill),
            icons::icon(if open {
                icons::CARET_DOWN
            } else {
                icons::CHEVRON_RIGHT
            })
            .size(typography::emphasis())
            .color(theme.neutral_front()),
        ]
        .align_y(Alignment::Center),
    )
    .padding([spacing::SPACE_6, spacing::SPACE_2])
    .width(Length::Fill)
    .style(button_style::bare(theme))
    .on_press(on_toggle);
    let mut section = column![header, table::divider_line(theme)].spacing(spacing::SPACE_6);
    if open {
        section = section.push(body.into());
    }
    section.into()
}

/// A soft primary-tinted bar spanning the width: a summary strip.
pub fn tinted_bar<'a, M: 'a>(theme: Theme, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(spacing::MD)
        .width(Length::Fill)
        .style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(
                theme.primary_tint(states::TINTED_PANEL_ALPHA),
            )),
            border: borders::card(theme, states::RADIUS_MD),
            ..container::Style::default()
        })
        .into()
}

/// A column of `items` (the right column, or the left one): scrolls on its
/// own, with the bar in its own gutter so it never covers the content.
pub fn scroll_pane<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    scrollable(
        container(content)
            .padding(scrollbar::gutter())
            .width(Length::Fill),
    )
    .direction(scrollbar::vertical())
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// A profile column beside a detail column, each scrolling on its own;
/// below the narrow breakpoint they stack and the page scrolls as one.
/// `width` is the width available to both.
pub fn master_detail<'a, M: 'a>(
    width: f32,
    master: Element<'a, M>,
    detail: Element<'a, M>,
) -> Element<'a, M> {
    if spacing::layout_class(width) == spacing::LayoutClass::Narrow {
        return scroll_pane(column![master, detail].spacing(spacing::MD));
    }
    row![
        container(scroll_pane(master)).width(Length::Fixed(spacing::profile_column_width(width))),
        scroll_pane(detail),
    ]
    .spacing(spacing::MD)
    .height(Length::Fill)
    .into()
}

/// What one timeline entry shows.
pub struct TimelineEntry {
    pub glyph: &'static str,
    pub tone: BadgeTone,
    pub title: String,
    pub description: String,
    pub amount: String,
    pub date: String,
}

/// One activity entry: a round glyph dot with a line running down to the
/// next entry (omitted on the `last`), then the title over a description
/// and, right-aligned, the amount over the date.
pub fn timeline_entry<'a, M: 'a>(theme: Theme, entry: TimelineEntry, last: bool) -> Element<'a, M> {
    let TimelineEntry {
        glyph,
        tone,
        title,
        description,
        amount,
        date,
    } = entry;
    let entry_height = spacing::grown(spacing::TIMELINE_ENTRY);
    let line_height = if last {
        0.0
    } else {
        entry_height - spacing::grown(spacing::TIMELINE_DOT)
    };
    let rail = column![
        badges::timeline_dot(theme, glyph, tone),
        container(Space::new(
            Length::Fixed(spacing::TIMELINE_LINE),
            Length::Fixed(line_height)
        ))
        .style(table::divider(theme)),
    ]
    .align_x(Alignment::Center);
    row![
        rail,
        column![
            labels::strong(theme, title, typography::body()).wrapping(Wrapping::None),
            labels::muted(theme, description, typography::caption()).wrapping(Wrapping::None),
        ]
        .spacing(spacing::SPACE_2)
        .width(Length::Fill),
        column![
            labels::amount(theme, amount, tone, typography::body()),
            labels::muted(theme, date, typography::caption()),
        ]
        .spacing(spacing::SPACE_2)
        .align_x(Alignment::End),
    ]
    .spacing(spacing::SM)
    .height(Length::Fixed(entry_height))
    .clip(true)
    .into()
}

/// A round icon button with a small caption under it: a quick action.
pub fn quick_action<'a, M: Clone + 'a>(
    theme: Theme,
    glyph: &'static str,
    label: impl IntoFragment<'a>,
    on_press: M,
) -> Element<'a, M> {
    column![
        button(
            icons::icon(glyph)
                .size(typography::emphasis())
                .width(Length::Fixed(spacing::icon_compact_size()))
                .align_x(Alignment::Center),
        )
        .padding(spacing::SPACE_8)
        .style(button_style::outlined(theme, false))
        .on_press(on_press),
        labels::muted(theme, label, typography::caption()),
    ]
    .spacing(spacing::SPACE_4)
    .align_x(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::components::badges::BadgeTone;

    #[test]
    fn a_card_face_tints_each_tone_differently_and_builds_in_both_themes() {
        use iced::Background;
        let fill = |theme, tone| {
            card_face_style(theme, tone)(&IcedTheme::Light)
                .background
                .map(|b| match b {
                    Background::Color(color) => color,
                    _ => theme.page_bg(),
                })
        };
        for theme in [Theme::Light, Theme::Dark] {
            assert_ne!(
                fill(theme, BadgeTone::Negative),
                fill(theme, BadgeTone::Positive)
            );
            for tone in [BadgeTone::Positive, BadgeTone::Negative] {
                let _: Element<'_, ()> = card_face(
                    theme,
                    tone,
                    CardFace {
                        caption: "Available",
                        figure: "PKR 1.00".into(),
                        line: "•••• 1234".into(),
                        bottom_left: "Bank",
                        bottom_right: "Debit",
                        height: Length::Fill,
                    },
                );
            }
        }
    }

    #[test]
    fn the_progress_bar_and_empty_state_build_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            for fraction in [-1.0, 0.0, 0.5, 1.0, 2.5] {
                let _: Element<'_, ()> = progress_bar(theme, fraction, BadgeTone::Warning);
            }
            let _: Element<'_, ()> = empty_state(
                theme,
                crate::frontend::components::icons::LANDMARK,
                "Nothing yet",
                Some("hint"),
                None,
            );
        }
    }
}
