use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::{App, DrillState, Screen};
use crate::music::{
    MAX_FRET, Note, STRING_LABELS, STRING_OPEN_MIDI, drill_pool, fret_positions,
};
use crate::pitch::PitchEvent;

const STAFF_X_START: f64 = 18.0;
const STAFF_X_END: f64 = 82.0;
const STAFF_LINE_Y_BOTTOM: f64 = 12.0;
const STAFF_LINE_Y_TOP: f64 = 28.0;
const NOTEHEAD_X: f64 = 50.0;
const NOTEHEAD_RADIUS: f64 = 1.6;

const LETTER_STEP: [i32; 12] = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6];
const IS_SHARP: [bool; 12] = [
    false, true, false, true, false, false, true, false, true, false, true, false,
];

fn staff_step(sounding_midi: i32) -> (i32, bool) {
    let written = sounding_midi + 12;
    let pc = written.rem_euclid(12) as usize;
    let octave = written.div_euclid(12) - 1;
    let step = 7 * octave + LETTER_STEP[pc];
    (step, IS_SHARP[pc])
}

fn staff_y(step: i32) -> f64 {
    2.0 * step as f64 - 24.0
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    match &app.screen {
        Screen::DevicePicker { devices, cursor } => {
            let items: Vec<ListItem> = devices
                .iter()
                .map(|(_, info)| {
                    let label = format!("{}  ({} channels)", info.name, info.max_input_channels);
                    ListItem::new(label)
                })
                .collect();
            draw_picker(
                frame,
                area,
                " Audio input device (enter=select, esc=quit) ",
                items,
                *cursor,
                if devices.is_empty() {
                    Some("No input devices found. Check audio interface connection.")
                } else {
                    None
                },
            );
        }
        Screen::ChannelPicker {
            device_name,
            channels,
            cursor,
            ..
        } => {
            let items: Vec<ListItem> = (0..*channels)
                .map(|c| ListItem::new(format!("Channel {} (input {})", c, c + 1)))
                .collect();
            let title = format!(
                " Channel on {} (enter=select, esc=back) ",
                truncate(device_name, 40)
            );
            draw_picker(frame, area, &title, items, *cursor, None);
        }
        Screen::KeyPicker {
            device_name,
            channel,
            keys,
            cursor,
            ..
        } => {
            let items: Vec<ListItem> = keys
                .iter()
                .map(|k| {
                    let pool_size = drill_pool(*k).len();
                    ListItem::new(format!("{}   ({} notes in range)", k, pool_size))
                })
                .collect();
            let title = format!(
                " Key  —  {}  ch{}  (enter=select, esc=back) ",
                truncate(device_name, 32),
                channel
            );
            draw_picker(frame, area, &title, items, *cursor, None);
        }
        Screen::Drill {
            device_name,
            channel,
            key,
            state,
            last_detected,
            ..
        } => {
            render_drill(
                frame,
                area,
                device_name,
                *channel,
                *key,
                state,
                last_detected.as_ref(),
            );
        }
        Screen::Transitioning => {}
    }
}

fn draw_picker(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: Vec<ListItem>,
    cursor: usize,
    empty_msg: Option<&str>,
) {
    let block = Block::default().title(title).borders(Borders::ALL);
    if items.is_empty() {
        let msg = empty_msg.unwrap_or("(none)");
        let para = Paragraph::new(msg)
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(para, area);
        return;
    }
    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(Some(cursor));
    frame.render_stateful_widget(list, area, &mut state);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn render_drill(
    frame: &mut Frame,
    area: Rect,
    device_name: &str,
    channel: u16,
    key: crate::music::Key,
    state: &DrillState,
    last_detected: Option<&PitchEvent>,
) {
    let (target, celebrating, hits) = match state {
        DrillState::Listening {
            target,
            consecutive_hits,
        } => (*target, false, *consecutive_hits),
        DrillState::Celebrating { note, .. } => (*note, true, crate::app::REQUIRED_HITS),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(area);

    let header_lines = vec![
        TextLine::from(vec![
            Span::styled("BASS TRAINER  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "key: {}   device: {}  ch{}",
                key,
                truncate(device_name, 28),
                channel
            )),
        ]),
        TextLine::from(if celebrating {
            Span::styled(
                format!("✓ correct — {}", target),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(format!(
                "play the note shown on the staff  (hits {}/{})",
                hits,
                crate::app::REQUIRED_HITS
            ))
        }),
    ];
    frame.render_widget(
        Paragraph::new(header_lines).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );

    render_staff(frame, chunks[1], target);

    if celebrating {
        render_tab(frame, chunks[2], Some(target));
    } else {
        render_tab(frame, chunks[2], None);
    }

    let footer_text = match last_detected {
        Some(evt) => format!(
            "heard: {} ({:+.1} cents @ {:.1} Hz, clarity {:.2})   esc/q to quit",
            evt.note, evt.cents, evt.frequency, evt.clarity
        ),
        None => "listening…   esc/q to quit".to_string(),
    };
    frame.render_widget(
        Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL)),
        chunks[3],
    );
}

fn render_staff(frame: &mut Frame, area: Rect, target: Note) {
    let (step, is_sharp) = staff_step(target.midi);
    let note_y = staff_y(step);

    let canvas = Canvas::default()
        .block(Block::default().title(" Bass clef ").borders(Borders::ALL))
        .marker(Marker::Braille)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 60.0])
        .paint(move |ctx| {
            for i in 0..5 {
                let y = STAFF_LINE_Y_BOTTOM + (i as f64) * 4.0;
                ctx.draw(&CanvasLine {
                    x1: STAFF_X_START,
                    y1: y,
                    x2: STAFF_X_END,
                    y2: y,
                    color: Color::Gray,
                });
            }

            if step < 18 {
                let mut s = 16;
                while s >= step {
                    let y = staff_y(s);
                    ctx.draw(&CanvasLine {
                        x1: NOTEHEAD_X - 4.0,
                        y1: y,
                        x2: NOTEHEAD_X + 4.0,
                        y2: y,
                        color: Color::Gray,
                    });
                    s -= 2;
                }
            } else if step > 26 {
                let mut s = 28;
                while s <= step {
                    let y = staff_y(s);
                    ctx.draw(&CanvasLine {
                        x1: NOTEHEAD_X - 4.0,
                        y1: y,
                        x2: NOTEHEAD_X + 4.0,
                        y2: y,
                        color: Color::Gray,
                    });
                    s += 2;
                }
            }

            ctx.draw(&Circle {
                x: NOTEHEAD_X,
                y: note_y,
                radius: NOTEHEAD_RADIUS,
                color: Color::White,
            });
            ctx.draw(&Circle {
                x: NOTEHEAD_X,
                y: note_y,
                radius: NOTEHEAD_RADIUS * 0.6,
                color: Color::White,
            });

            ctx.print(
                STAFF_X_START - 8.0,
                (STAFF_LINE_Y_BOTTOM + STAFF_LINE_Y_TOP) / 2.0,
                TextLine::from(Span::styled("?:", Style::default().fg(Color::Yellow))),
            );

            if is_sharp {
                ctx.print(
                    NOTEHEAD_X - 6.0,
                    note_y,
                    TextLine::from(Span::styled(
                        "#",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )),
                );
            }

            ctx.print(
                STAFF_X_END + 4.0,
                note_y,
                TextLine::from(Span::styled(
                    format!("{}", target),
                    Style::default().add_modifier(Modifier::DIM),
                )),
            );
        });

    frame.render_widget(canvas, area);
}

fn render_tab(frame: &mut Frame, area: Rect, reveal_for: Option<Note>) {
    let mut lines: Vec<TextLine> = Vec::with_capacity(4);
    for s in (0..4).rev() {
        let label = STRING_LABELS[s];
        let open_midi = STRING_OPEN_MIDI[s];
        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(format!("{}|", label)));
        for fret in 0..=MAX_FRET {
            let is_target = reveal_for
                .map(|n| n.midi - open_midi == fret && (0..=MAX_FRET).contains(&fret))
                .unwrap_or(false);
            if is_target {
                let txt = if fret >= 10 {
                    format!("{}", fret)
                } else {
                    format!(" {}", fret)
                };
                spans.push(Span::styled(
                    txt,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw("--"));
            }
        }
        spans.push(Span::raw("|"));
        lines.push(TextLine::from(spans));
    }

    let title = match reveal_for {
        Some(n) => {
            let count = fret_positions(n).len();
            format!(" Tab — {}  ({} positions) ", n, count)
        }
        None => " Tab ".to_string(),
    };

    let para = Paragraph::new(lines).block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staff_step_for_open_strings() {
        assert_eq!(staff_step(28).0, 16);
        assert_eq!(staff_step(43).0, 25);
    }

    #[test]
    fn staff_step_for_c4_middle_c() {
        assert_eq!(staff_step(60).0, 35);
    }

    #[test]
    fn staff_step_marks_sharps() {
        assert!(staff_step(30).1);
        assert!(!staff_step(28).1);
        assert!(!staff_step(29).1);
    }
}
