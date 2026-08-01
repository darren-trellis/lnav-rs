use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::columns::{self, Segment, SegmentKind};
use crate::highlight;
use crate::model::LogEntry;
use crate::text;
use crate::theme::{Theme, Tone};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.hidden_count() > 0 {
        format!(
            " {} · {} filtered ",
            app.source.display_name(),
            app.hidden_count()
        )
    } else {
        format!(" {} ", app.source.display_name())
    };
    let block = {
        let theme = &app.theme;
        let list_focused = app.is_list_focused();
        let border_tone = if list_focused {
            theme.window_focus_border
        } else {
            theme.border
        };
        let mut border_style = theme.tone_fg_style(border_tone);
        if list_focused {
            border_style = border_style.add_modifier(Modifier::BOLD);
        }
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .style(
                Style::default()
                    .bg(theme.background)
                    .fg(theme.foreground.fg),
            )
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (content, bar_area) = super::split_scrollbar(inner, app.config.scrollbar);
    app.pointer.hit.list_inner = content;
    app.pointer.hit.list_scrollbar = bar_area.unwrap_or_default();
    let viewport = content.height as usize;
    app.ensure_visible(viewport, app.config.scroll_moves_selection);

    let (pin_rows, sep_rows, body_h) = app.list_band_layout(viewport);
    app.pointer.hit.list_pin_rows = pin_rows;

    if app.display_len() == 0 {
        let theme = &app.theme;
        let msg = if app.source.len() == 0 {
            " waiting for log lines… "
        } else {
            " no lines match filters (:filter clear / :filter off) "
        };
        let empty = Paragraph::new(Line::from(Span::styled(
            msg,
            theme.tone_style(theme.dim, theme.background),
        )))
        .style(Style::default().bg(theme.background));
        frame.render_widget(empty, content);
        return;
    }

    let body_end = (app.view.scroll + body_h).min(app.view.visible.len());
    let mut lines =
        Vec::with_capacity(pin_rows + sep_rows + body_end.saturating_sub(app.view.scroll));

    let show_nums = app.config.line_numbers || app.config.relative_line_numbers;
    let line_no_width = if show_nums {
        let abs_w = app.display_len().max(1).to_string().len();
        let rel_w = app
            .view
            .selected
            .max(
                app.display_len()
                    .saturating_sub(1)
                    .saturating_sub(app.view.selected),
            )
            .max(1)
            .to_string()
            .len();
        if app.config.line_numbers && app.config.relative_line_numbers {
            abs_w.max(rel_w)
        } else if app.config.relative_line_numbers {
            rel_w
        } else {
            abs_w
        }
    } else {
        0
    };

    let ts_fmt = app.config.timestamp_format.as_str();
    let mut measure_rows: Vec<(&crate::model::LogEntry, usize)> = Vec::new();
    for display in 0..pin_rows {
        let src = app.view.pinned[display];
        measure_rows.push((&app.source.entries()[src], display + 1));
    }
    for body_idx in app.view.scroll..body_end {
        let display = app.pin_count() + body_idx;
        let src = app.view.visible[body_idx];
        measure_rows.push((&app.source.entries()[src], display + 1));
    }
    let col_widths = columns::measure_widths(&app.config.columns, &measure_rows, ts_fmt);

    for display in 0..pin_rows {
        let src = app.view.pinned[display];
        let entry = &app.source.entries()[src];
        let selected = display == app.view.selected && app.is_list_focused();
        let gutter_num = line_number_label(app, display);
        lines.push(render_line(
            app,
            entry,
            LineRenderOptions {
                view_line: display + 1,
                col_widths: &col_widths,
                gutter_num: gutter_num.as_deref(),
                line_no_width,
                selected,
                pinned: app.is_display_pinned(display),
                width: content.width as usize,
            },
        ));
    }
    if sep_rows > 0 {
        let theme = &app.theme;
        let width = content.width as usize;
        let label = " pinned ";
        let rule = if width <= label.len() {
            "─".repeat(width)
        } else {
            let side = (width - label.len()) / 2;
            format!(
                "{}{}{}",
                "─".repeat(side),
                label,
                "─".repeat(width - side - label.len())
            )
        };
        lines.push(Line::from(Span::styled(
            rule,
            theme.tone_style(theme.dim, theme.background),
        )));
    }
    for body_idx in app.view.scroll..body_end {
        let display = app.pin_count() + body_idx;
        let src = app.view.visible[body_idx];
        let entry = &app.source.entries()[src];
        let selected = display == app.view.selected && app.is_list_focused();
        let gutter_num = line_number_label(app, display);
        lines.push(render_line(
            app,
            entry,
            LineRenderOptions {
                view_line: display + 1,
                col_widths: &col_widths,
                gutter_num: gutter_num.as_deref(),
                line_no_width,
                selected,
                pinned: app.is_display_pinned(display),
                width: content.width as usize,
            },
        ));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(app.theme.background));
    frame.render_widget(paragraph, content);

    if let Some(bar) = bar_area {
        let theme = &app.theme;
        super::draw_scrollbar(
            frame,
            bar,
            app.view.visible.len(),
            app.view.scroll,
            body_h.max(1),
            theme.tone_fg_style(theme.window_focus_border),
            theme.tone_fg_style(theme.dim),
        );
    }
}

fn line_number_label(app: &App, display: usize) -> Option<String> {
    let abs = app.config.line_numbers;
    let rel = app.config.relative_line_numbers;
    if !abs && !rel {
        return None;
    }
    let selected = display == app.view.selected;
    let text = if rel && !(abs && selected) {
        // Pure relative, or hybrid on non-current lines.
        if selected && !abs {
            "0".into()
        } else {
            display.abs_diff(app.view.selected).to_string()
        }
    } else {
        (display + 1).to_string()
    };
    Some(text)
}

struct LineRenderOptions<'a> {
    view_line: usize,
    col_widths: &'a [usize],
    gutter_num: Option<&'a str>,
    line_no_width: usize,
    selected: bool,
    pinned: bool,
    width: usize,
}

fn render_line<'a>(app: &'a App, entry: &'a LogEntry, options: LineRenderOptions<'_>) -> Line<'a> {
    let LineRenderOptions {
        view_line,
        col_widths,
        gutter_num,
        line_no_width,
        selected,
        pinned,
        width,
    } = options;
    let theme = &app.theme;
    let gutter = if selected {
        "▌"
    } else if pinned {
        "▀"
    } else {
        " "
    };

    let default_border = columns::ColumnBorderStyle {
        width: theme.column_border_width,
        padding: theme.column_border_padding,
    };
    let segments = columns::render_segments_sized(
        &app.config.columns,
        col_widths,
        entry,
        &columns::FormatOptions {
            timestamp_format: &app.config.timestamp_format,
            view_line,
        },
        default_border,
    );

    let gutter_style = if selected {
        theme.selection_style()
    } else {
        theme.tone_style(theme.dim, theme.background)
    };

    let mut spans = vec![Span::styled(gutter.to_string(), gutter_style)];
    let mut used = 1usize;

    if let Some(num_text) = gutter_num {
        let num = format!("{num_text:>width$}", width = line_no_width);
        let num_style = if selected {
            theme.selection_style()
        } else {
            theme.tone_style(theme.dim, theme.background)
        };
        used += UnicodeWidthStr::width(num.as_str());
        spans.push(Span::styled(num, num_style));

        let border_style = app
            .config
            .columns
            .first()
            .map(|col| columns::ColumnBorderStyle::resolve(col, default_border))
            .unwrap_or(default_border);
        let border = columns::column_separator(border_style);
        let border_w = UnicodeWidthStr::width(border.text.as_str());
        if used + border_w <= width {
            let style = segment_style(theme, entry, &border, selected);
            spans.push(Span::styled(border.text, style));
            used += border_w;
        }
    }

    let search = app.search.regex.as_ref();
    for segment in segments {
        if used >= width {
            break;
        }
        let budget = width - used;
        let text = text::truncate_width(&segment.text, budget);
        let text_w = UnicodeWidthStr::width(text.as_str());
        if text.is_empty() && !segment.text.is_empty() {
            break;
        }
        let base = segment_style(theme, entry, &segment, selected);
        let match_style = theme.search_highlight_style(row_bg(theme, selected));
        // Only highlight list text when search targets the list (not details).
        let re = if app.search.in_details { None } else { search };
        highlight::push_highlighted(&mut spans, text, base, match_style, re);
        used += text_w;
    }

    // Pad to full width so the selection highlight spans the whole row.
    if used < width {
        let pad_style = if selected {
            theme.selection_style()
        } else {
            Style::default().bg(theme.background)
        };
        spans.push(Span::styled(" ".repeat(width - used), pad_style));
    }

    Line::from(spans)
}

fn row_bg(theme: &Theme, selected: bool) -> Color {
    if selected {
        theme.selection_bg
    } else {
        theme.background
    }
}

fn segment_style(theme: &Theme, entry: &LogEntry, segment: &Segment, selected: bool) -> Style {
    let row_bg = row_bg(theme, selected);

    if segment.kind == SegmentKind::Level {
        return apply_tone(
            theme,
            theme.level_color(entry.level),
            selected,
            row_bg,
            true,
        );
    }

    // Selected row: keep border fg, use selection bg so the highlight is continuous.
    if segment.kind == SegmentKind::ColumnBorder {
        if selected {
            return Style::default()
                .fg(theme.column_border.fg)
                .bg(theme.selection_bg);
        }
        return apply_tone(theme, theme.column_border, false, row_bg, false);
    }

    let tone = match segment.kind {
        SegmentKind::Level | SegmentKind::ColumnBorder => unreachable!(),
        SegmentKind::Timestamp => theme.timestamp,
        SegmentKind::Message | SegmentKind::Raw => theme.foreground,
        SegmentKind::LineNo | SegmentKind::Format => theme.dim,
        SegmentKind::Field => theme.key,
        SegmentKind::Literal => theme.dim,
    };

    apply_tone(theme, tone, selected, row_bg, false)
}

fn apply_tone(theme: &Theme, tone: Tone, selected: bool, row_bg: Color, bold: bool) -> Style {
    if let Some(bg) = tone.bg {
        let mut style = Style::default().fg(tone.fg).bg(bg);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        return style;
    }
    if selected {
        return theme.selection_style().bg(row_bg);
    }
    let mut style = Style::default().fg(tone.fg).bg(row_bg);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}
