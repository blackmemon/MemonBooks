use super::{
    badges, borders, button as button_style, colors::Theme, fonts, icons, labels, panels, spacing,
    states, tabs, typography,
};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Element, Length};

/// A reusable underlined tab strip for form sections. Only one page of
/// tabs is shown (the page holding the active tab); a trailing "More"
/// button jumps to the first tab of the next page, wrapping back to the
/// start from the last page. No scrollbar is ever drawn.
pub fn tab_bar<'a, Message: Clone + 'a>(
    theme: Theme,
    labels: &'a [&'static str],
    active: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    const TABS_PER_PAGE: usize = 3;
    let page = active / TABS_PER_PAGE;
    let start = page * TABS_PER_PAGE;
    let end = (start + TABS_PER_PAGE).min(labels.len());
    let has_more = labels.len() > TABS_PER_PAGE;
    let more_target = if end < labels.len() { end } else { 0 };

    let mut tab_row = row![].spacing(spacing::SPACE_8).align_y(Alignment::Center);
    for index in start..end {
        let selected = index == active;
        let mut label_text = text(labels[index])
            .size(typography::label())
            .color(tabs::text_color(theme, selected));
        if selected {
            label_text = label_text.font(fonts::BOLD);
        }
        tab_row = tab_row.push(
            column![
                button(label_text)
                    .style(tabs::tab_button(theme))
                    .padding([spacing::SPACE_6, spacing::SPACE_10])
                    .on_press(on_select(index)),
                container(Space::with_height(Length::Fixed(2.0)))
                    .width(Length::Fill)
                    .style(tabs::underline(theme, selected)),
            ]
            .spacing(spacing::SPACE_2)
            .width(Length::Shrink),
        );
    }
    tab_row = tab_row.push(Space::with_width(Length::Fill));
    if has_more {
        let (caption, glyph) = if end < labels.len() {
            ("More", icons::CHEVRON_RIGHT)
        } else {
            ("Back", icons::CHEVRON_LEFT)
        };
        tab_row = tab_row.push(
            button(
                row![
                    text(caption).size(typography::label()),
                    icons::icon(glyph).size(typography::label()),
                ]
                .spacing(spacing::SPACE_4)
                .align_y(Alignment::Center),
            )
            .style(button_style::outlined(theme, false))
            .padding([spacing::SPACE_4, spacing::SPACE_8])
            .on_press(on_select(more_target)),
        );
    }
    tab_row.width(Length::Fill).into()
}

/// An icon-labelled value card (two fit side by side), echoing the
/// "vitals" style field layout: a small icon chip, a muted label above
/// the value.
pub fn field_box<'a, Message: 'a>(
    theme: Theme,
    icon: &'static str,
    label: &'a str,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    let chip = container(
        icons::icon(icon)
            .size(typography::body())
            .color(theme.primary()),
    )
    .padding(spacing::SPACE_8)
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(
            theme.primary_tint(states::SELECTED_ALPHA),
        )),
        border: iced::Border {
            radius: states::RADIUS_SM.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    container(
        row![
            chip,
            column![labels::muted(theme, label, typography::caption()), value,]
                .spacing(spacing::SPACE_4)
                .width(Length::Fill),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(spacing::SM)
    .width(Length::FillPortion(1))
    .style(panels::flat_card(theme, states::RADIUS_MD))
    .into()
}

pub struct DetailPanel<'a, Message, Select> {
    pub theme: Theme,
    pub title: &'a str,
    pub tab_labels: &'a [&'static str],
    pub active_tab: usize,
    pub on_select_tab: Select,
    pub summary: Option<Element<'a, Message>>,
    pub content: Element<'a, Message>,
    pub footer: Element<'a, Message>,
    pub on_preview: Message,
}

/// Shared shell for a right-side detail editor with tabs, preview, and actions.
pub fn detail_panel<'a, Message: Clone + 'a, Select: Fn(usize) -> Message + 'a>(
    config: DetailPanel<'a, Message, Select>,
) -> Element<'a, Message> {
    let DetailPanel {
        theme,
        title,
        tab_labels,
        active_tab,
        on_select_tab,
        summary,
        content,
        footer,
        on_preview,
    } = config;
    let header = row![
        text(title)
            .size(typography::panel_title())
            .font(fonts::BOLD)
            .color(theme.neutral_font()),
        Space::with_width(Length::Fill),
        button(text("Preview").size(typography::label()))
            .padding([spacing::SPACE_6, spacing::SPACE_10])
            .style(button_style::outlined(theme, false))
            .on_press(on_preview),
    ]
    .spacing(spacing::SPACE_8)
    .align_y(Alignment::Center);

    let mut body = column![header].spacing(spacing::MD);
    if let Some(summary) = summary {
        body = body.push(summary);
    }
    body = body
        .push(tab_bar(theme, tab_labels, active_tab, on_select_tab))
        .push(
            scrollable(
                container(content)
                    .padding(iced::Padding {
                        right: spacing::SCROLLBAR_WIDTH * 2.0,
                        ..iced::Padding::default()
                    })
                    .width(Length::Fill),
            )
            .direction(iced::widget::scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(spacing::SCROLLBAR_WIDTH)
                    .scroller_width(spacing::SCROLLBAR_WIDTH),
            ))
            .height(Length::Fill),
        )
        .push(footer);

    container(body)
        .padding(spacing::MD)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panels::flat_card(theme, states::RADIUS_LG))
        .into()
}

#[derive(Debug, Clone)]
pub struct PreviewField {
    pub label: String,
    pub value: String,
}

impl PreviewField {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreviewSection {
    pub title: String,
    pub fields: Vec<PreviewField>,
}

impl PreviewSection {
    pub fn new(title: impl Into<String>, fields: Vec<PreviewField>) -> Self {
        Self {
            title: title.into(),
            fields,
        }
    }
}

pub struct RecordPreview {
    pub name: String,
    pub subtitle: String,
    pub status: String,
    pub identity: Vec<PreviewField>,
    pub sections: Vec<PreviewSection>,
}

/// Reusable full-page profile preview, modeled after a master-record overview.
pub fn record_preview<'a, Message: Clone + 'a>(
    theme: Theme,
    preview: RecordPreview,
    on_back: Message,
    on_edit: Message,
) -> Element<'a, Message> {
    let RecordPreview {
        name,
        subtitle,
        status: status_label,
        identity,
        sections,
    } = preview;
    let initials: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    let initials = if initials.is_empty() {
        "?".to_string()
    } else {
        initials
    };
    let tone = if status_label == "Active" {
        badges::BadgeTone::Positive
    } else {
        badges::BadgeTone::Neutral
    };
    let avatar = container(
        text(initials)
            .size(typography::subtitle())
            .font(fonts::BOLD)
            .color(theme.primary()),
    )
    .center_x(Length::Fixed(64.0))
    .center_y(Length::Fixed(64.0))
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(
            theme.primary_tint(states::SELECTED_ALPHA),
        )),
        border: iced::Border {
            radius: 32.0.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    let status = container(
        text(status_label)
            .size(typography::caption())
            .font(fonts::BOLD)
            .color(badges::tone_colors(theme, tone).1),
    )
    .padding([spacing::SPACE_2, spacing::SPACE_10])
    .style(badges::pill(theme, tone));

    let mut identity_lines = column![].spacing(spacing::SPACE_6);
    for field in identity {
        identity_lines = identity_lines.push(
            row![
                text(field.label)
                    .size(typography::caption())
                    .color(theme.neutral_front()),
                text(field.value)
                    .size(typography::caption())
                    .color(theme.neutral_font()),
            ]
            .spacing(spacing::SPACE_8),
        );
    }
    let hero = container(
        row![
            avatar,
            column![
                row![
                    text(name)
                        .size(typography::subtitle())
                        .font(fonts::BOLD)
                        .color(theme.neutral_font()),
                    status,
                ]
                .spacing(spacing::SPACE_10)
                .align_y(Alignment::Center),
                text(subtitle)
                    .size(typography::caption())
                    .color(theme.neutral_front()),
                identity_lines,
            ]
            .spacing(spacing::SPACE_6)
            .width(Length::Fill),
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(
            theme.primary_tint(states::TINTED_PANEL_ALPHA),
        )),
        border: borders::card(theme, states::RADIUS_LG),
        ..container::Style::default()
    });

    let mut content = column![
        row![
            button(text("Back").size(typography::label()).font(fonts::BOLD),)
                .padding([spacing::SM, spacing::MD])
                .style(button_style::outlined(theme, false))
                .on_press(on_back),
            Space::with_width(Length::Fill),
            button(text("Edit").size(typography::label()).font(fonts::BOLD))
                .padding([spacing::SM, spacing::MD])
                .style(button_style::mode(theme, true))
                .on_press(on_edit),
        ]
        .spacing(spacing::SPACE_8)
        .align_y(Alignment::Center),
        hero,
        text("Overview")
            .size(typography::panel_title())
            .font(fonts::BOLD)
            .color(theme.neutral_font()),
    ]
    .spacing(spacing::MD)
    .width(Length::Fill);

    for section in sections {
        let mut fields = column![].spacing(spacing::SPACE_6);
        for field in section.fields {
            fields = fields.push(
                row![
                    text(field.label)
                        .size(typography::caption())
                        .color(theme.neutral_front())
                        .width(Length::FillPortion(1)),
                    text(if field.value.is_empty() {
                        "--".to_string()
                    } else {
                        field.value
                    })
                    .size(typography::body())
                    .color(theme.neutral_font())
                    .width(Length::FillPortion(2)),
                ]
                .spacing(spacing::MD),
            );
        }
        content = content.push(
            container(
                column![
                    text(section.title)
                        .size(typography::emphasis())
                        .font(fonts::BOLD)
                        .color(theme.primary()),
                    fields,
                ]
                .spacing(spacing::SPACE_10),
            )
            .padding(spacing::MD)
            .width(Length::Fill)
            .style(panels::flat_card(theme, states::RADIUS_MD)),
        );
    }

    scrollable(content.padding(spacing::SPACE_20))
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new()))
        .height(Length::Fill)
        .into()
}
