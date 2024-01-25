use ndoc::rope_utils::{get_line_info, tab2space_char_idx};
use ndoc::{rope_utils, Rope};
use ratatui::buffer::Cell;
use ratatui::prelude::*;
use ratatui::widgets::StatefulWidget;

use ndoc::Selection;

#[derive(Debug, Default)]
pub struct TextArea {
    pub rope: Rope,
    pub sel: Vec<Selection>,
    pub indent_len: usize,
}

#[derive(Debug, Default)]
pub struct TextAreaState {
    pub scrollx: usize,
    pub scrolly: usize,
}

impl StatefulWidget for TextArea {
    type State = TextAreaState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut y = 0;
        for line in self.rope.lines().skip(state.scrolly).take(area.height as _)
        //.map(|l| Line::raw(get_line_info(&l,0,4)))
        {
            let line = Line::raw(get_line_info(&line, 0, self.indent_len));
            buf.set_line(0, y, &line, area.width);
            y += 1;
        }

        for s in self.sel {
            let selection_area = s.areas(&self.rope);
            let mut y = (s.start().line - state.scrolly) as u16;
            for line in selection_area {
                let xs = tab2space_char_idx(&self.rope.slice(..), y as _, self.indent_len)[line.0];
                let xe = tab2space_char_idx(&self.rope.slice(..), y as _, self.indent_len)[line.1];
                let w = xe - xs;
                let bgarea = Rect::new(xs as _, y, w as _, 1);
                if y < area.height {
                    buf.set_style(bgarea, Style::new().bg(Color::Rgb(12, 48, 128)));
                }
                y += 1;
            }
            if s.head.line >= state.scrolly && s.head.line < state.scrolly + area.height as usize {
                buf.get_mut(
                    tab2space_char_idx(&self.rope.slice(..), s.head.line as _, 4)[s.head.column]
                        as _,
                    s.head.line as _,
                )
                .modifier |= Modifier::UNDERLINED;
            }
        }
    }
}
