use super::{
    button as button_style, colors::Theme, fonts, icons, inputs, labels, panels, spacing, states,
    table, tabs, typography,
};
use iced::widget::text::IntoFragment;
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input, Space, TextInput,
};
use iced::{Alignment, Background, Element, Length};

/// A labelled detail-form text input in the shared input style.
pub fn text_field<'a, Message: Clone + 'a>(
    theme: Theme,
    label: &'a str,
    input: TextInput<'a, Message>,
) -> Element<'a, Message> {
    field_row(
        theme,
        label,
        input
            .style(inputs::text(theme))
            .size(typography::form_input())
            .width(Length::Fill)
            .padding(spacing::MD)
            .into(),
    )
}

/// A labelled value shown as plain text.
pub fn readonly_field<'a, Message: 'a>(
    theme: Theme,
    label: &'a str,
    value: impl IntoFragment<'a>,
) -> Element<'a, Message> {
    field_row(
        theme,
        label,
        text(value)
            .size(typography::form_input())
            .color(theme.neutral_font())
            .into(),
    )
}

/// A labelled input that is shown but cannot be typed into.
pub fn disabled_field<'a, Message: Clone + 'a>(
    theme: Theme,
    label: &'a str,
    value: &'a str,
) -> Element<'a, Message> {
    text_field(theme, label, text_input("", value))
}

/// A labelled multi-line input.
pub fn text_area_field<'a, Message: Clone + 'a>(
    theme: Theme,
    label: &'a str,
    content: &'a text_editor::Content,
    on_action: impl Fn(text_editor::Action) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        theme,
        label,
        text_editor(content)
            .placeholder(label)
            .style(inputs::text_edit(theme))
            .size(typography::form_input())
            .height(Length::Fixed(spacing::TEXT_AREA_HEIGHT))
            .padding(spacing::MD)
            .on_action(on_action)
            .into(),
    )
}

/// Lays `fields` out `columns` to a line; a short last line is padded so
/// its fields keep the same width as the ones above.
pub fn field_grid<'a, Message: 'a>(
    fields: Vec<Element<'a, Message>>,
    columns: usize,
) -> Element<'a, Message> {
    let mut grid = column![].spacing(spacing::MD).width(Length::Fill);
    let mut remaining = fields.into_iter();
    loop {
        let mut line = row![].spacing(spacing::SM).width(Length::Fill);
        let mut placed = 0;
        for field in remaining.by_ref().take(columns) {
            line = line.push(field);
            placed += 1;
        }
        if placed == 0 {
            break;
        }
        for _ in placed..columns {
            line = line.push(Space::with_width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    grid.into()
}

/// The card at the top of a detail panel: the record's name, a muted
/// line under it, and a badge on the right.
pub fn summary_card<'a, Message: 'a>(
    theme: Theme,
    title: impl IntoFragment<'a>,
    subtitle: impl IntoFragment<'a>,
    badge: Element<'a, Message>,
) -> Element<'a, Message> {
    panels::padded(
        theme,
        row![
            column![
                labels::strong(theme, title, typography::emphasis()),
                labels::muted(theme, subtitle, typography::caption()),
            ]
            .spacing(spacing::SPACE_4)
            .width(Length::Fill),
            badge,
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center),
    )
}

/// A bordered block in a detail form: a bold title with an optional
/// action on the right, then the block's rows, or `empty` when there are
/// none.
fn form_block<'a, Message: 'a>(
    theme: Theme,
    title: &'a str,
    action: Element<'a, Message>,
    rows: Vec<Element<'a, Message>>,
    empty: &'a str,
) -> Element<'a, Message> {
    let header = row![
        labels::muted(theme, title, typography::body() + 1.0).font(fonts::BOLD),
        Space::with_width(Length::Fill),
        action,
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    let body: Element<'a, Message> = if rows.is_empty() {
        labels::muted(theme, empty, typography::body()).into()
    } else {
        column(rows).spacing(spacing::SM).into()
    };
    panels::padded(theme, column![header, body].spacing(spacing::SPACE_10))
}

/// A compact text input for a row inside a [`form_block`].
fn row_input<'a, Message: Clone + 'a>(
    theme: Theme,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .style(inputs::text(theme))
        .size(typography::form_input())
        .padding(spacing::SM)
}

/// Editor for user-defined name/value pairs: one row of two inputs and a
/// remove button per field, and an "Add Field" button.
pub struct CustomFields<'a, Message, Name, Value, Remove> {
    pub theme: Theme,
    pub title: &'a str,
    /// Each field's name and value, in order.
    pub fields: Vec<(&'a str, &'a str)>,
    pub on_name: Name,
    pub on_value: Value,
    pub on_remove: Remove,
    pub on_add: Message,
}

pub fn custom_fields<'a, Message, Name, Value, Remove>(
    config: CustomFields<'a, Message, Name, Value, Remove>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    Name: Fn(usize, String) -> Message + Clone + 'a,
    Value: Fn(usize, String) -> Message + Clone + 'a,
    Remove: Fn(usize) -> Message + 'a,
{
    let CustomFields {
        theme,
        title,
        fields,
        on_name,
        on_value,
        on_remove,
        on_add,
    } = config;
    let rows = fields
        .into_iter()
        .enumerate()
        .map(|(index, (name, value))| {
            let on_name = on_name.clone();
            let on_value = on_value.clone();
            let name_input = row_input(theme, "Field name", name, move |input| {
                on_name(index, input)
            });
            let value_input = row_input(theme, "Value", value, move |input| on_value(index, input));
            row![
                name_input.width(Length::FillPortion(2)),
                value_input.width(Length::FillPortion(3)),
                button_style::icon_tool(theme, icons::REMOVE, "Remove", on_remove(index), false),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into()
        })
        .collect();
    let add = button_style::icon_label(
        icons::ADD,
        "Add Field",
        [spacing::SPACE_6, spacing::SPACE_14],
    )
    .style(button_style::outlined(theme, false))
    .on_press(on_add)
    .into();
    form_block(theme, title, add, rows, "No custom fields")
}

/// One attached file as [`attachments`] lists it.
pub struct AttachmentRow<'a> {
    pub name: &'a str,
    pub size_bytes: u64,
}

/// The attached files of a record: a row per file with its name and
/// size plus Rename and Remove buttons, and an "Add Attachment" button.
/// While `renaming` names a row, that row shows an input holding the
/// draft name instead, confirmed with Enter or the tick button.
pub struct Attachments<'a, Message, Edit, Rename, Remove> {
    pub theme: Theme,
    pub title: &'a str,
    pub files: Vec<AttachmentRow<'a>>,
    /// The row being renamed and its draft name.
    pub renaming: Option<(usize, &'a str)>,
    pub on_add: Message,
    pub on_edit: Edit,
    pub on_rename: Rename,
    pub on_rename_done: Message,
    pub on_remove: Remove,
}

pub fn attachments<'a, Message, Edit, Rename, Remove>(
    config: Attachments<'a, Message, Edit, Rename, Remove>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    Edit: Fn(usize) -> Message + 'a,
    Rename: Fn(String) -> Message + 'a,
    Remove: Fn(usize) -> Message + 'a,
{
    let Attachments {
        theme,
        title,
        files,
        renaming,
        on_add,
        on_edit,
        on_rename,
        on_rename_done,
        on_remove,
    } = config;
    let mut on_rename = Some(on_rename);
    let mut rows = Vec::with_capacity(files.len());
    for (index, file) in files.into_iter().enumerate() {
        let glyph = icons::icon(icons::FILE)
            .size(typography::emphasis())
            .color(theme.primary());
        let draft = renaming.and_then(|(row, draft)| (row == index).then_some(draft));
        let (body, action): (Element<'a, Message>, Element<'a, Message>) =
            match (draft, on_rename.take_if(|_| draft.is_some())) {
                (Some(draft), Some(on_rename)) => (
                    row_input(theme, "Display name", draft, on_rename)
                        .on_submit(on_rename_done.clone())
                        .width(Length::Fill)
                        .into(),
                    button_style::icon_tool(
                        theme,
                        icons::CHECK,
                        "Done",
                        on_rename_done.clone(),
                        false,
                    ),
                ),
                _ => (
                    column![
                        labels::strong(theme, file.name, typography::label()),
                        labels::muted(theme, file_size(file.size_bytes), typography::caption()),
                    ]
                    .spacing(spacing::SPACE_2)
                    .width(Length::Fill)
                    .into(),
                    button_style::icon_tool(theme, icons::EDIT, "Rename", on_edit(index), false),
                ),
            };
        rows.push(
            row![
                glyph,
                body,
                action,
                button_style::icon_tool(theme, icons::REMOVE, "Remove", on_remove(index), false),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into(),
        );
    }
    let add = button_style::icon_label(
        icons::ATTACH,
        "Add Attachment",
        [spacing::SPACE_6, spacing::SPACE_14],
    )
    .style(button_style::outlined(theme, false))
    .on_press(on_add)
    .into();
    form_block(theme, title, add, rows, "No attachments")
}

/// A file size for people: bytes, KB or MB with one decimal.
pub fn file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f < KB {
        format!("{bytes} B")
    } else if bytes_f < KB * KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{:.1} MB", bytes_f / (KB * KB))
    }
}

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
    const TABS_PER_PAGE: usize = 5;
    let page = active / TABS_PER_PAGE;
    let start = page * TABS_PER_PAGE;
    let end = (start + TABS_PER_PAGE).min(labels.len());
    let has_more = labels.len() > TABS_PER_PAGE;
    let more_target = if end < labels.len() { end } else { 0 };

    let mut tab_row = row![].spacing(spacing::SPACE_8).align_y(Alignment::Center);
    for (index, label) in labels.iter().enumerate().take(end).skip(start) {
        let selected = index == active;
        let mut label_text = text(*label)
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

/// A labelled value card (two fit side by side), with a muted label above
/// the value.
pub fn field_box<'a, Message: 'a>(
    theme: Theme,
    label: &'a str,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        column![labels::muted(theme, label, typography::caption()), value,]
            .spacing(spacing::SPACE_2)
            .width(Length::Fill),
    )
    .padding(spacing::XS)
    .width(Length::FillPortion(1))
    .style(panels::flat_card(theme, states::RADIUS_MD))
    .into()
}

/// A field with no icon: the label sits to the left of the value, in a
/// fixed-width column so the values line up down the form.
pub fn field_row<'a, Message: 'a>(
    theme: Theme,
    label: &'a str,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        row![
            container(labels::muted(theme, label, typography::body() + 1.0).font(fonts::BOLD))
                .width(Length::Fixed(spacing::FIELD_LABEL_WIDTH_130)),
            value,
        ]
        .spacing(spacing::XS)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(spacing::XS)
    .width(Length::FillPortion(1))
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(theme.page_bg())),
        ..container::Style::default()
    })
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
        .push(table::divider_line(theme))
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

    panels::side(theme, body)
}
