use iced::widget::button::{Status, Style};
use iced::widget::text::{IntoFragment, Wrapping};
use iced::widget::{button as button_widget, container, row, text, tooltip, Button};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, icons, panels, spacing, states, typography};

/// A button with a bold, single-line label; callers add the style and
/// the press.
pub fn bold<'a, M: 'a>(label: impl IntoFragment<'a>, padding: impl Into<Padding>) -> Button<'a, M> {
    button_widget(
        text(label)
            .size(typography::label())
            .font(fonts::BOLD)
            .wrapping(Wrapping::None),
    )
    .padding(padding)
}

/// A bold-labelled button with an icon before the label.
pub fn icon_label<'a, M: 'a>(
    glyph: &'static str,
    label: impl IntoFragment<'a>,
    padding: impl Into<Padding>,
) -> Button<'a, M> {
    button_widget(
        row![
            icons::icon(glyph).size(typography::label()),
            text(label)
                .size(typography::label())
                .font(fonts::BOLD)
                .wrapping(Wrapping::None),
        ]
        .spacing(spacing::SPACE_6)
        .align_y(Alignment::Center),
    )
    .padding(padding)
}

/// Small square icon-only button with a hover tooltip: outlined when
/// `bordered`, bare (tinted on hover) otherwise.
pub fn icon_tool<'a, M: Clone + 'a>(
    theme: Theme,
    glyph: &'static str,
    tip: &'a str,
    on_press: M,
    bordered: bool,
) -> Element<'a, M> {
    tooltip(
        button_widget(icons::icon(glyph).size(typography::emphasis()).center())
            .width(Length::Fixed(spacing::ICON_BUTTON_SIZE))
            .height(Length::Fixed(spacing::ICON_BUTTON_SIZE))
            .style(outlined_or_plain(theme, bordered, false))
            .on_press(on_press),
        container(text(tip).size(typography::caption()))
            .padding([spacing::SPACE_4, spacing::SPACE_8])
            .style(panels::flat_card(theme, states::RADIUS_MD)),
        tooltip::Position::Bottom,
    )
    .into()
}

/// [`outlined`] (selected when `active`) when `bordered`, [`plain`]
/// otherwise -- for buttons that switch between the two looks.
pub fn outlined_or_plain(
    theme: Theme,
    bordered: bool,
    active: bool,
) -> impl Fn(&IcedTheme, Status) -> Style {
    let outlined = outlined(theme, active);
    let plain = plain(theme);
    move |iced_theme: &IcedTheme, status: Status| {
        if bordered {
            outlined(iced_theme, status)
        } else {
            plain(iced_theme, status)
        }
    }
}

/// A button with no background, border or padding of its own -- for a
/// clickable label such as a sortable column header.
pub fn bare(_theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    |_iced_theme: &IcedTheme, _status: Status| Style::default()
}

pub fn neutral(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.neutral_bg_hover(),
            _ => theme.neutral_bg(),
        };
        let border_color = match status {
            Status::Hovered | Status::Pressed => theme.neutral_border_hover(),
            _ => theme.neutral_border(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.neutral_font(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                color: border_color,
                radius: states::RADIUS_SM.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn primary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.primary_hover(),
            _ => theme.primary(),
        };
        Style {
            background: Some(Background::Color(background)),
            text_color: if matches!(status, Status::Hovered | Status::Pressed) {
                theme.accent_front()
            } else {
                theme.positive_front()
            },
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn secondary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.secondary_hover(),
            _ => theme.secondary(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: if matches!(status, Status::Hovered | Status::Pressed) {
                theme.accent_front()
            } else {
                theme.negative_front()
            },
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn mode(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let (normal, hover) = match active {
            true => (theme.primary(), theme.primary_hover()),
            false => (theme.secondary(), theme.secondary_hover()),
        };
        let background = match status {
            Status::Hovered | Status::Pressed => hover,
            _ => normal,
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn positive(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.positive_bg_hover(),
            _ => theme.positive_bg(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn negative(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.negative_bg_hover(),
            _ => theme.negative_bg(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn destructive(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Disabled => theme.neutral_bg(),
            Status::Hovered | Status::Pressed => Color {
                a: states::SOLID_BUTTON_HOVER_ALPHA,
                ..theme.negative_front()
            },
            Status::Active => theme.negative_front(),
        };
        let text_color = if matches!(status, Status::Disabled) {
            theme.neutral_front()
        } else {
            theme.accent_front()
        };
        Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn plain(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if matches!(status, Status::Hovered) {
            Some(Background::Color(theme.neutral_bg()))
        } else {
            None
        };
        Style {
            background,
            text_color: theme.neutral_front(),
            border: Border {
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn outline_primary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => Color {
                a: states::OUTLINE_BUTTON_HOVER_ALPHA,
                ..theme.primary()
            },
            _ => theme.transparent(),
        };
        Style {
            background: Some(Background::Color(background)),
            text_color: theme.primary(),
            border: Border {
                color: theme.primary(),
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn nav_item(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if active {
            Color {
                a: theme.tint_alpha(states::SELECTED_ALPHA),
                ..theme.primary()
            }
        } else if matches!(status, Status::Hovered) {
            Color {
                a: states::GHOST_ROW_HOVER_ALPHA,
                ..theme.neutral_front()
            }
        } else {
            theme.transparent()
        };
        let text_color = if active {
            theme.primary()
        } else {
            theme.neutral_front()
        };
        Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                radius: states::RADIUS_MD.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn profile_card(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if matches!(status, Status::Hovered) {
            Color {
                a: states::NEUTRAL_CONTROL_HOVER_ALPHA,
                ..theme.neutral_bg()
            }
        } else {
            theme.neutral_bg()
        };
        Style {
            background: Some(Background::Color(background)),
            text_color: theme.text_primary(),
            border: Border {
                radius: states::RADIUS_MD.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// Surface-colored button with a light border; the border and text turn
/// primary on hover. `active` draws it as the selected state (tinted
/// primary), e.g. the current page box in pagination. Dimmed when disabled.
pub fn outlined(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let accent = active || matches!(status, Status::Hovered | Status::Pressed);
        let dim = |color: Color| {
            if matches!(status, Status::Disabled) {
                color.scale_alpha(theme.disabled_alpha())
            } else {
                color
            }
        };
        Style {
            background: Some(Background::Color(if active {
                theme.primary_tint(states::SELECTED_ALPHA)
            } else {
                theme.surface()
            })),
            text_color: dim(if accent {
                theme.primary()
            } else {
                theme.neutral_font()
            }),
            border: Border {
                color: dim(if accent {
                    theme.primary()
                } else {
                    theme.neutral_border()
                }),
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// The outlined "Back" button that leaves an inline page view.
pub fn back<'a, M: Clone + 'a>(theme: Theme, on_press: M) -> Element<'a, M> {
    icon_label(
        icons::ARROW_LEFT,
        "Back",
        [spacing::SPACE_6, spacing::SPACE_14],
    )
    .style(outlined(theme, false))
    .on_press(on_press)
    .into()
}

/// A clickable card: the page surface faintly tinted with `tone`'s
/// color, a hairline border and a soft shadow; hovering draws the border
/// in the primary color and lifts the shadow.
pub fn tile(
    theme: Theme,
    tone: crate::frontend::components::badges::BadgeTone,
) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let hovered = matches!(status, Status::Hovered | Status::Pressed);
        let (_, tint) = crate::frontend::components::badges::tone_colors(theme, tone);
        let mut border = crate::frontend::components::borders::card(theme, states::RADIUS_LG);
        if hovered {
            border.color = theme.primary();
        }
        let mut shadow = crate::frontend::components::shadows::card(theme);
        if hovered {
            shadow.blur_radius *= 2.0;
        }
        Style {
            background: Some(Background::Color(Color {
                a: theme.tint_alpha(states::TINTED_PANEL_ALPHA),
                ..tint
            })),
            text_color: theme.neutral_font(),
            border,
            shadow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::components::badges::BadgeTone;

    #[test]
    fn a_tile_turns_its_border_primary_on_hover_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let style = tile(theme, BadgeTone::Positive);
            let idle = style(&IcedTheme::Light, Status::Active);
            let hover = style(&IcedTheme::Light, Status::Hovered);
            assert_ne!(idle.border.color, theme.primary());
            assert_eq!(hover.border.color, theme.primary());
            assert!(hover.shadow.blur_radius > idle.shadow.blur_radius);
        }
    }
}
