use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::columns::{self, Segment, SegmentKind};
use crate::model::LogEntry;
use crate::theme::{Theme, Tone};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let viewport = inner.height as usize;
    app.ensure_visible(viewport);

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
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border.fg))
            .title(title)
            .style(
                Style::default()
                    .bg(theme.background)
                    .fg(theme.foreground.fg),
            )
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.hit.list_inner = inner;

    if app.visible.is_empty() {
        let theme = &app.theme;
        let msg = if app.source.len() == 0 {
            " waiting for log lines… "
        } else {
            " no lines match filters (:clear-filters / :filter off) "
        };
        let empty = Paragraph::new(Line::from(Span::styled(
            msg,
            theme.tone_style(theme.dim, theme.background),
        )))
        .style(Style::default().bg(theme.background));
        frame.render_widget(empty, inner);
        return;
    }

    let end = (app.scroll + viewport).min(app.visible.len());
    let mut lines = Vec::with_capacity(end - app.scroll);

    let show_nums = app.config.line_numbers || app.config.relative_line_numbers;
    let line_no_width = if show_nums {
        let abs_w = app.visible.len().max(1).to_string().len();
        let rel_w = app
            .selected
            .max(app.visible.len().saturating_sub(1).saturating_sub(app.selected))
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
    let measure_rows: Vec<(&crate::model::LogEntry, usize)> = (app.scroll..end)
        .map(|vis_idx| {
            let src = app.visible[vis_idx];
            (&app.source.entries()[src], vis_idx + 1)
        })
        .collect();
    let col_widths = columns::measure_widths(&app.config.columns, &measure_rows, ts_fmt);

    for vis_idx in app.scroll..end {
        let src = app.visible[vis_idx];
        let entry = &app.source.entries()[src];
        let selected = vis_idx == app.selected;
        let is_match = app.search_matches.binary_search(&vis_idx).is_ok();
        let gutter_num = line_number_label(app, vis_idx);
        lines.push(render_line(
            app,
            entry,
            vis_idx + 1,
            &col_widths,
            gutter_num.as_deref(),
            line_no_width,
            selected,
            is_match,
            inner.width as usize,
        ));
    }

    let paragraph =
        Paragraph::new(lines).style(Style::default().bg(app.theme.background));
    frame.render_widget(paragraph, inner);
}

fn line_number_label(app: &App, vis_idx: usize) -> Option<String> {
    let abs = app.config.line_numbers;
    let rel = app.config.relative_line_numbers;
    if !abs && !rel {
        return None;
    }
    let selected = vis_idx == app.selected;
    let text = if rel && !(abs && selected) {
        // Pure relative, or hybrid on non-current lines.
        if selected && !abs {
            "0".into()
        } else {
            vis_idx.abs_diff(app.selected).to_string()
        }
    } else {
        (vis_idx + 1).to_string()
    };
    Some(text)
}

fn render_line<'a>(
    app: &'a App,
    entry: &'a LogEntry,
    view_line: usize,
    col_widths: &[usize],
    gutter_num: Option<&str>,
    line_no_width: usize,
    selected: bool,
    is_match: bool,
    width: usize,
) -> Line<'a> {
    let theme = &app.theme;
    let gutter = if selected { "▌" } else { " " };

    let segments = columns::render_segments_sized(
        &app.config.columns,
        col_widths,
        entry,
        &columns::FormatOptions {
            timestamp_format: &app.config.timestamp_format,
            view_line,
        },
    );

    let gutter_style = if selected {
        theme.selection_style()
    } else {
        theme.tone_style(theme.dim, theme.background)
    };

    let mut spans = vec![Span::styled(gutter.to_string(), gutter_style)];
    let mut used = 1usize;

    if let Some(num_text) = gutter_num {
        let num = format!("{num_text:>width$} ", width = line_no_width);
        let num_style = if selected {
            theme.selection_style()
        } else {
            theme.tone_style(theme.dim, theme.background)
        };
        used += UnicodeWidthStr::width(num.as_str());
        spans.push(Span::styled(num, num_style));
    }

    for segment in segments {
        if used >= width {
            break;
        }
        let budget = width - used;
        let text = truncate(&segment.text, budget);
        let text_w = UnicodeWidthStr::width(text.as_str());
        if text.is_empty() && !segment.text.is_empty() {
            break;
        }
        let style = segment_style(theme, entry, &segment, selected, is_match);
        spans.push(Span::styled(text, style));
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

fn segment_style(
    theme: &Theme,
    entry: &LogEntry,
    segment: &Segment,
    selected: bool,
    is_match: bool,
) -> Style {
    let row_bg = if selected {
        theme.selection_bg
    } else {
        theme.background
    };

    if segment.kind == SegmentKind::Level {
        return apply_tone(
            theme,
            theme.level_color(entry.level),
            selected,
            row_bg,
            true,
        );
    }

    if is_match && matches!(segment.kind, SegmentKind::Message | SegmentKind::Raw) {
        return apply_tone(theme, theme.search_match, selected, row_bg, false);
    }

    let tone = match segment.kind {
        SegmentKind::Level => unreachable!(),
        SegmentKind::Timestamp => theme.timestamp,
        SegmentKind::Message | SegmentKind::Raw => theme.foreground,
        SegmentKind::LineNo | SegmentKind::Format => theme.dim,
        SegmentKind::Field => theme.key,
        SegmentKind::Literal => theme.dim,
    };

    apply_tone(theme, tone, selected, row_bg, false)
}

fn apply_tone(
    theme: &Theme,
    tone: Tone,
    selected: bool,
    row_bg: Color,
    bold: bool,
) -> Style {
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

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        if w + cw + 1 > max {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
