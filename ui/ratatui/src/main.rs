mod text_area;

use crossterm::{
    event::{self, KeyCode, KeyEventKind, KeyModifiers, EnableMouseCapture, DisableMouseCapture, MouseEvent, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, execute,
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
    io::{stdout, Result, self},
};
use text_area::{TextArea, TextAreaState};
use ndoc::position_to_char;

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

    let mut stdout = io::stdout();
    
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
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
                    indent_len: document.file_info.indentation.len()
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
                        position_to_char(&document.rope.slice(..),s.start()),
                        position_to_char(&document.rope.slice(..),s.end()),
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


        match event::read()? {
        
            event::Event::Key(KeyEvent {kind: KeyEventKind::Press, code, modifiers, ..}) => {
                match code {
                    KeyCode::Esc => break,

                    KeyCode::Up
                        if modifiers
                            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        document.duplicate_selection(MoveDirection::Up)
                    }
                    KeyCode::Down
                        if modifiers
                            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        document.duplicate_selection(MoveDirection::Down)
                    }
                    KeyCode::Up => document.move_selections(
                        MoveDirection::Up,
                        modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Down => document.move_selections(
                        MoveDirection::Down,
                        modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Left if modifiers.contains(KeyModifiers::CONTROL) => document
                        .move_selections_word(
                            MoveDirection::Left,
                            modifiers.contains(KeyModifiers::SHIFT),
                        ),
                    KeyCode::Left => document.move_selections(
                        MoveDirection::Left,
                        modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => document
                        .move_selections_word(
                            MoveDirection::Right,
                            modifiers.contains(KeyModifiers::SHIFT),
                        ),
                    KeyCode::Right => document.move_selections(
                        MoveDirection::Right,
                        modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::PageUp => {
                        // shift only work if the terminal don't intercep it
                        document.page_up(
                            area.height as _,
                            modifiers.contains(KeyModifiers::SHIFT),
                        );
                    }
                    KeyCode::PageDown => document.page_down(
                        area.height as _,
                        modifiers.contains(KeyModifiers::SHIFT),
                    ),
                    KeyCode::Home => document.home(modifiers.contains(KeyModifiers::SHIFT)),
                    KeyCode::End => document.end(modifiers.contains(KeyModifiers::SHIFT)),
                    KeyCode::Backspace => {
                        document.backspace();
                    }
                    KeyCode::Delete => {
                        document.delete();
                    }
                    KeyCode::Enter => document.insert(&document.file_info.linefeed.to_string()),
                    KeyCode::Tab => document.indent(false),

                    KeyCode::BackTab => document.deindent(),
                    KeyCode::Char('z') if modifiers.contains(KeyModifiers::CONTROL) => document.undo(),
                    KeyCode::Char('y') if modifiers.contains(KeyModifiers::CONTROL) => document.redo(),
                    KeyCode::Char(c) => {
                        document.insert(&c.to_string());
                    },
                    
                    _ => (),
                }
            },
            // event::Event::Mouse(MouseEvent { kind, column, row, modifiers }) => (
            //     match kind {
            //         event::MouseEventKind::ScrollDown => 
            //     }
            // ),
            _ => (),
        }
    }

    execute!(terminal.backend_mut(), LeaveAlternateScreen,DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}
