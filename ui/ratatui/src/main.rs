mod text_area;

use crossterm::{
    event::{self, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ndoc::{Document, MoveDirection};
use gumdrop::Options;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    prelude::{CrosstermBackend, Stylize, Terminal},
    text::Span,
    widgets::Paragraph,
    Frame,
};
use std::{
    fs,
    io::{stdout, Result},
};
use text_area::{TextArea, TextAreaState};

#[derive(Options, Debug)]

struct AppOptions {
    #[options(free)]
    files: Vec<String>,
}

fn main() -> Result<()> {
    let opts = AppOptions::parse_args_default_or_exit();

    let mut document = if opts.files.len() > 0 {
        Document::from_file(&opts.files[0])?
    } else {
        Document::default()
    };

    let mut text_area_state = TextAreaState::default();

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    loop {
        let mut area = Default::default();

        terminal.draw(|frame| {
            area = frame.size();
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);

            frame.render_stateful_widget(
                TextArea {
                    rope: document.rope.clone(),
                    sel: document.selections.clone(),
                },
                chunks[0],
                &mut text_area_state,
            );
            // Debug
            frame.render_widget(
                Paragraph::new(
                    document
                        .selections
                        .iter()
                        .map(|s| format!("[{},{}]", s.head.line, s.head.column))
                        .collect::<Vec<String>>()
                        .join(","),
                ),
                chunks[1],
            );
            let flattened_content = document
                .rope
                .chars()
                .map(|c| match c {
                    '\n' => "↵".to_owned(),
                    '\r' => "↵".to_owned(),
                    c => c.to_string(),
                })
                .collect::<Vec<String>>()
                .join("");
            frame.render_widget(Paragraph::new(flattened_content), chunks[2]);
            let flattened_selection = document
                .selections
                .iter()
                .flat_map(|s| {
                    vec![
                        s.start().char_idx(&document.rope.slice(..)),
                        s.end().char_idx(&document.rope.slice(..)),
                    ]
                })
                .collect::<Vec<usize>>();
            let mut s = String::with_capacity(document.rope.chars().count());
            for i in 0..document.rope.chars().count() + 1 {
                if flattened_selection.contains(&i) {
                    s.push('^');
                } else {
                    s.push(' ');
                }
            }
            frame.render_widget(Paragraph::new(s), chunks[3]);
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => break,

                    KeyCode::Up
                        if key
                            .modifiers
                            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        document.duplicate_selection(MoveDirection::Up)
                    }
                    KeyCode::Down
                        if key
                            .modifiers
                            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        document.duplicate_selection(MoveDirection::Down)
                    }
                    KeyCode::Up => document.move_selections(
                        MoveDirection::Up,
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Down => document.move_selections(
                        MoveDirection::Down,
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => document
                        .move_selections_word(
                            MoveDirection::Left,
                            key.modifiers.contains(KeyModifiers::SHIFT),
                        ),
                    KeyCode::Left => document.move_selections(
                        MoveDirection::Left,
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => document
                        .move_selections_word(
                            MoveDirection::Right,
                            key.modifiers.contains(KeyModifiers::SHIFT),
                        ),
                    KeyCode::Right => document.move_selections(
                        MoveDirection::Right,
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::PageUp => {
                        // shift only work if the terminal don't intercep it
                        document.page_up(
                            area.height as _,
                            key.modifiers.contains(KeyModifiers::SHIFT),
                        );
                    }
                    KeyCode::PageDown => document.page_down(
                        area.height as _,
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Home => document.home(key.modifiers.contains(KeyModifiers::SHIFT)),
                    KeyCode::End => document.end(key.modifiers.contains(KeyModifiers::SHIFT)),
                    KeyCode::Backspace => {
                        document.backspace();
                    }
                    KeyCode::Delete => {
                        document.delete();
                    }
                    KeyCode::Enter => document.insert(&document.file_info.linefeed.to_string()),
                    KeyCode::Tab => document.indent(false),

                    KeyCode::BackTab => document.deindent(),
                    KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => document.undo(),
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => document.redo(),
                    KeyCode::Char(c) => {
                        document.insert(&c.to_string());
                    }
                    _ => (),
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
