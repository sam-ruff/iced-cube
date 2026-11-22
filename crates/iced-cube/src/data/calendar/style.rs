//! Colours of the calendar's parts, resolved from the theme's tokens.

use iced::widget::button::Status;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow};

use super::entry::Tone;
use crate::theme::{Tokens, fade, mix, on, radius};

/// The colour a [`Tone`] stands for.
pub fn tone(tokens: &Tokens, tone: Tone) -> Color {
    match tone {
        Tone::Primary => tokens.primary,
        Tone::Neutral => tokens.muted_foreground,
        Tone::Success => tokens.success,
        Tone::Warning => tokens.warning,
        Tone::Destructive => tokens.destructive,
    }
}

/// The colours of an entry, as a bar in the month or a block in the day.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chip {
    pub background: Color,
    pub foreground: Color,
    /// The stripe down its leading edge.
    pub stripe: Color,
}

/// An entry is a wash of its tone with a solid stripe, deeper under the
/// pointer. The selected entry is filled with the tone. A disabled entry
/// looks like one at rest.
pub fn chip(tokens: &Tokens, entry_tone: Tone, selected: bool, status: Status) -> Chip {
    let colour = tone(tokens, entry_tone);
    if selected {
        return Chip {
            background: colour,
            foreground: on(colour),
            stripe: colour,
        };
    }
    let wash = match status {
        Status::Active | Status::Disabled => 0.16,
        Status::Hovered => 0.26,
        Status::Pressed => 0.34,
    };
    // A calendar without a handler is read-only, not unavailable, so its
    // entries keep their full colour.
    Chip {
        background: mix(tokens.background, colour, wash),
        foreground: tokens.foreground,
        stripe: colour,
    }
}

/// The button style of an entry drawn with [`chip`] colours.
pub fn chip_button(
    tokens: &Tokens,
    entry_tone: Tone,
    selected: bool,
    status: Status,
) -> button::Style {
    let chip = chip(tokens, entry_tone, selected, status);
    button::Style {
        background: Some(Background::Color(chip.background)),
        text_color: chip.foreground,
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// What a day in the month grid is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Day {
    pub today: bool,
    pub selected: bool,
    /// Whether the day belongs to the month on show.
    pub in_month: bool,
}

/// The colours of a day's number: a filled pill for today.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayNumber {
    pub background: Option<Color>,
    pub foreground: Color,
}

pub fn day_number(tokens: &Tokens, day: Day) -> DayNumber {
    if day.today {
        return DayNumber {
            background: Some(tokens.primary),
            foreground: tokens.primary_foreground,
        };
    }
    DayNumber {
        background: None,
        foreground: if day.in_month {
            tokens.foreground
        } else {
            tokens.muted_foreground
        },
    }
}

/// The surface of a day cell, which is a button. The selected day takes
/// the accent with a ring, a day under the pointer a lighter accent, and
/// days outside the month sit back.
pub fn cell(tokens: &Tokens, day: Day, status: Status) -> button::Style {
    let hovered = matches!(status, Status::Hovered | Status::Pressed);
    let background = if day.selected {
        Some(tokens.accent)
    } else if hovered {
        Some(mix(tokens.background, tokens.accent, 0.6))
    } else if !day.in_month {
        Some(mix(tokens.background, tokens.muted, 0.4))
    } else {
        None
    };
    button::Style {
        background: background.map(Background::Color),
        text_color: day_number(tokens, day).foreground,
        border: Border {
            color: if day.selected {
                tokens.ring
            } else {
                Color::TRANSPARENT
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// A day in the year view: a circle filled for today, ringed when
/// selected and tinted under the pointer.
pub fn year_day(tokens: &Tokens, day: Day, status: Status) -> button::Style {
    let hovered = matches!(status, Status::Hovered | Status::Pressed);
    let number = day_number(tokens, day);
    let background = if day.today {
        number.background
    } else if day.selected || hovered {
        Some(tokens.accent)
    } else {
        None
    };
    button::Style {
        background: background.map(Background::Color),
        text_color: number.foreground,
        border: Border {
            color: if day.selected {
                tokens.ring
            } else {
                Color::TRANSPARENT
            },
            width: 1.0,
            radius: radius::FULL.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// A row in the agenda, or a day in the strip above a narrow week.
pub fn row(tokens: &Tokens, selected: bool, hovered: bool) -> container::Style {
    let background = if selected {
        Some(tokens.accent)
    } else if hovered {
        Some(mix(tokens.background, tokens.accent, 0.6))
    } else {
        None
    };
    container::Style {
        background: background.map(Background::Color),
        border: Border {
            radius: radius::MD.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The frame around a view.
pub fn frame(tokens: &Tokens) -> container::Style {
    container::Style {
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::LG.into(),
        },
        ..container::Style::default()
    }
}

/// A timed entry in the month, which is text beside a dot: no fill at
/// rest, the accent under the pointer and the tone when selected.
pub fn entry_line(
    tokens: &Tokens,
    entry_tone: Tone,
    selected: bool,
    status: Status,
) -> button::Style {
    if selected {
        return chip_button(tokens, entry_tone, true, status);
    }
    let background = match status {
        Status::Hovered | Status::Pressed => Some(tokens.accent),
        Status::Active | Status::Disabled => None,
    };
    button::Style {
        background: background.map(Background::Color),
        text_color: tokens.foreground,
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// A line between days, weeks or hours.
pub fn line(tokens: &Tokens) -> Color {
    tokens.border
}

/// The line showing the current time.
pub fn now_line(tokens: &Tokens) -> Color {
    tokens.destructive
}

/// The outline of an entry being dragged out, moved or resized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preview {
    pub background: Color,
    pub border: Color,
}

pub fn preview(tokens: &Tokens) -> Preview {
    Preview {
        background: fade(tokens.primary, 0.16),
        border: tokens.primary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn every_tone_has_its_own_colour() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let colours: Vec<_> = Tone::ALL.iter().map(|t| tone(&tokens, *t)).collect();
            for (index, colour) in colours.iter().enumerate() {
                assert!(!colours[..index].contains(colour), "{:?}", Tone::ALL[index]);
            }
        }
    }

    #[test]
    fn a_chip_is_a_wash_that_deepens_under_the_pointer() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for entry_tone in Tone::ALL {
                let rest = chip(&tokens, entry_tone, false, Status::Active);
                let hovered = chip(&tokens, entry_tone, false, Status::Hovered);
                let pressed = chip(&tokens, entry_tone, false, Status::Pressed);
                assert_eq!(rest.stripe, tone(&tokens, entry_tone));
                assert_eq!(rest.foreground, tokens.foreground);
                assert_ne!(rest.background, tokens.background);
                assert_ne!(rest.background, hovered.background);
                assert_ne!(hovered.background, pressed.background);
                assert_ne!(rest.background, rest.stripe);
            }
        }
    }

    #[test]
    fn a_selected_chip_is_filled_and_keeps_its_text_readable() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for entry_tone in Tone::ALL {
                let colour = tone(&tokens, entry_tone);
                for status in [Status::Active, Status::Hovered, Status::Pressed] {
                    let selected = chip(&tokens, entry_tone, true, status);
                    assert_eq!(selected.background, colour);
                    assert_eq!(selected.foreground, on(colour));
                }
            }
        }
    }

    #[test]
    fn a_read_only_calendar_keeps_its_entries_at_full_colour() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for entry_tone in Tone::ALL {
                assert_eq!(
                    chip(&tokens, entry_tone, false, Status::Disabled),
                    chip(&tokens, entry_tone, false, Status::Active)
                );
                assert_eq!(
                    entry_line(&tokens, entry_tone, false, Status::Disabled),
                    entry_line(&tokens, entry_tone, false, Status::Active)
                );
            }
        }
    }

    #[test]
    fn a_timed_entry_in_the_month_is_plain_until_hovered_or_selected() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for entry_tone in Tone::ALL {
                let rest = entry_line(&tokens, entry_tone, false, Status::Active);
                assert_eq!(rest.background, None);
                assert_eq!(rest.text_color, tokens.foreground);

                for status in [Status::Hovered, Status::Pressed] {
                    let hovered = entry_line(&tokens, entry_tone, false, status);
                    assert_eq!(hovered.background, Some(Background::Color(tokens.accent)));
                }

                let selected = entry_line(&tokens, entry_tone, true, Status::Active);
                assert_eq!(
                    selected,
                    chip_button(&tokens, entry_tone, true, Status::Active)
                );
            }
        }
    }

    #[test]
    fn the_chip_button_uses_the_chip_colours() {
        let tokens = Tokens::of(&dark());
        let colours = chip(&tokens, Tone::Warning, false, Status::Hovered);
        let style = chip_button(&tokens, Tone::Warning, false, Status::Hovered);
        assert_eq!(
            style.background,
            Some(Background::Color(colours.background))
        );
        assert_eq!(style.text_color, colours.foreground);
        assert_eq!(style.border.width, 0.0);
    }

    #[test]
    fn today_is_a_filled_pill_and_other_months_sit_back() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let today = day_number(
                &tokens,
                Day {
                    today: true,
                    in_month: true,
                    selected: false,
                },
            );
            assert_eq!(today.background, Some(tokens.primary));
            assert_eq!(today.foreground, tokens.primary_foreground);

            let inside = day_number(
                &tokens,
                Day {
                    in_month: true,
                    ..Day::default()
                },
            );
            let outside = day_number(&tokens, Day::default());
            assert_eq!(inside.background, None);
            assert_eq!(inside.foreground, tokens.foreground);
            assert_eq!(outside.foreground, tokens.muted_foreground);
        }
    }

    #[test]
    fn cells_show_selection_hover_and_other_months() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let inside = Day {
                in_month: true,
                ..Day::default()
            };
            let rest = cell(&tokens, inside, Status::Active);
            assert_eq!(rest.background, None);
            assert_eq!(rest.border.color, Color::TRANSPARENT);
            assert_eq!(cell(&tokens, inside, Status::Disabled), rest);

            let hovered = cell(&tokens, inside, Status::Hovered);
            assert!(hovered.background.is_some());
            assert_eq!(cell(&tokens, inside, Status::Pressed), hovered);

            let selected = cell(
                &tokens,
                Day {
                    selected: true,
                    ..inside
                },
                Status::Hovered,
            );
            assert_eq!(selected.background, Some(Background::Color(tokens.accent)));
            assert_eq!(selected.border.color, tokens.ring);
            assert_ne!(selected.background, hovered.background);

            let outside = cell(&tokens, Day::default(), Status::Active);
            assert!(outside.background.is_some());
            assert_ne!(outside.background, selected.background);
            assert_eq!(outside.text_color, tokens.muted_foreground);
        }
    }

    #[test]
    fn days_of_the_year_mark_today_selection_and_hover() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let plain = Day {
                in_month: true,
                ..Day::default()
            };
            let rest = year_day(&tokens, plain, Status::Active);
            assert_eq!(rest.background, None);
            assert_eq!(rest.text_color, tokens.foreground);
            assert_eq!(year_day(&tokens, plain, Status::Disabled), rest);

            let hovered = year_day(&tokens, plain, Status::Hovered);
            assert_eq!(hovered.background, Some(Background::Color(tokens.accent)));
            assert_eq!(year_day(&tokens, plain, Status::Pressed), hovered);

            let today = year_day(
                &tokens,
                Day {
                    today: true,
                    ..plain
                },
                Status::Hovered,
            );
            assert_eq!(today.background, Some(Background::Color(tokens.primary)));
            assert_eq!(today.text_color, tokens.primary_foreground);

            let selected = year_day(
                &tokens,
                Day {
                    selected: true,
                    ..plain
                },
                Status::Active,
            );
            assert_eq!(selected.border.color, tokens.ring);
            assert_eq!(rest.border.color, Color::TRANSPARENT);
        }
    }

    #[test]
    fn rows_show_selection_and_hover() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(row(&tokens, false, false).background, None);
            let hovered = row(&tokens, false, true);
            let selected = row(&tokens, true, false);
            assert!(hovered.background.is_some());
            assert_eq!(selected.background, Some(Background::Color(tokens.accent)));
            assert_ne!(hovered.background, selected.background);
            assert_eq!(row(&tokens, true, true), selected);
        }
    }

    #[test]
    fn a_custom_theme_reaches_every_part() {
        use crate::theme::Config;
        use iced::color;

        let theme = Config::light()
            .primary(color!(0x6d28d9))
            .accent(color!(0xede9fe))
            .border(color!(0xddd6fe))
            .success(color!(0x0d9488))
            .warning(color!(0xea580c))
            .destructive(color!(0xe11d48))
            .foreground(color!(0x1e1b2e))
            .build();
        let tokens = Tokens::of(&theme);

        assert_eq!(tone(&tokens, Tone::Primary), color!(0x6d28d9));
        assert_eq!(tone(&tokens, Tone::Success), color!(0x0d9488));
        assert_eq!(tone(&tokens, Tone::Warning), color!(0xea580c));
        assert_eq!(tone(&tokens, Tone::Destructive), color!(0xe11d48));
        assert_eq!(
            chip(&tokens, Tone::Primary, true, Status::Active).background,
            color!(0x6d28d9)
        );
        assert_eq!(
            chip(&tokens, Tone::Success, false, Status::Active).foreground,
            color!(0x1e1b2e)
        );

        let today = Day {
            today: true,
            in_month: true,
            selected: false,
        };
        assert_eq!(
            day_number(&tokens, today).background,
            Some(color!(0x6d28d9))
        );
        assert_eq!(
            year_day(&tokens, today, Status::Active).background,
            Some(Background::Color(color!(0x6d28d9)))
        );

        let selected = Day {
            selected: true,
            in_month: true,
            today: false,
        };
        assert_eq!(
            cell(&tokens, selected, Status::Active).background,
            Some(Background::Color(color!(0xede9fe)))
        );
        assert_eq!(
            row(&tokens, true, false).background,
            Some(Background::Color(color!(0xede9fe)))
        );
        assert_eq!(
            entry_line(&tokens, Tone::Primary, false, Status::Hovered).background,
            Some(Background::Color(color!(0xede9fe)))
        );

        assert_eq!(line(&tokens), color!(0xddd6fe));
        assert_eq!(frame(&tokens).border.color, color!(0xddd6fe));
        assert_eq!(now_line(&tokens), color!(0xe11d48));
        assert_eq!(preview(&tokens).border, color!(0x6d28d9));
    }

    #[test]
    fn lines_frames_and_previews_come_from_the_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(line(&tokens), tokens.border);
            assert_eq!(now_line(&tokens), tokens.destructive);
            assert_eq!(frame(&tokens).border.color, tokens.border);
            let preview = preview(&tokens);
            assert_eq!(preview.border, tokens.primary);
            assert!(preview.background.a < 0.5);
        }
    }
}
