use once_cell::sync::{Lazy, OnceCell};
use ropey::Rope;
use std::{
    ops::{Deref, Range},
    sync::{Arc, Mutex},
};
use syntect::{
    highlighting::{HighlightState, Highlighter, RangedHighlightIterator, Style, ThemeSet},
    parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet},
};

use crate::rope_utils;

pub static SYNTAXSET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
pub static THEMESET: OnceCell<ThemeSet> = OnceCell::new();

pub struct ThemeSetRegistry;

impl ThemeSetRegistry {
    pub fn get() -> &'static ThemeSet {
        THEMESET.get_or_init(ThemeSet::load_defaults)
    }
}


#[derive(Debug)]
pub struct StateCache {
    states: Vec<(ParseState, HighlightState)>,
    highlighter: Highlighter<'static>,
}
#[derive(Debug, Clone)]
pub struct SpanStyle {
    pub style: Style,
    pub range: Range<usize>,
}

impl SpanStyle {
    pub fn new(style: Style, range: Range<usize>) -> Self {
        Self { style, range }
    }
}

#[derive(Debug, Clone)]
pub struct StyledLine {
    styles: Vec<SpanStyle>,
}

impl StyledLine {
    pub fn new(styles: Vec<SpanStyle>) -> Self {
        Self { styles }
    }
}

impl Deref for StyledLine {
    type Target = Vec<SpanStyle>;

    fn deref(&self) -> &Self::Target {
        &self.styles
    }
}

#[derive(Debug, Clone)]
pub struct StyledLinesCache {
    pub lines: Arc<Mutex<Vec<StyledLine>>>,
}

impl StyledLinesCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, line_idx: usize) -> Option<StyledLine> {
        self.lines.lock().unwrap().get(line_idx).cloned()
    }
}

impl Default for StyledLinesCache {
    fn default() -> Self {
        Self {
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for StateCache {
    fn default() -> Self {
        StateCache {
            states: Vec::new(),
            highlighter: Highlighter::new(&ThemeSetRegistry::get().themes["base16-ocean.dark"]),
        }
    }
}

impl StateCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn change_theme(&mut self, theme: &str) {
        if !ThemeSetRegistry::get().themes.contains_key(theme) {
            // TODO: logerror
            return;
        }
        self.highlighter = Highlighter::new(&ThemeSetRegistry::get().themes[theme]);
    }

    pub fn update_range(
        &mut self,
        highlighted_line: &StyledLinesCache,
        syntax: &SyntaxReference,
        rope: &Rope,
        start: usize,
        end: usize,
        tab_len: usize,
    ) {
        // states are cached every 16 lines
        let start = (start >> 4).min(self.states.len());
        let end = (end.min(rope.len_lines()) >> 4) + 1;

        self.states.truncate(start);

        let mut states = self.states.last().cloned().unwrap_or_else(|| {
            (
                ParseState::new(syntax),
                HighlightState::new(&self.highlighter, ScopeStack::new()),
            )
        });

        for i in start << 4..(end << 4).min(rope.len_lines()) {
            let str = rope_utils::get_line_info(&rope.slice(..), i, tab_len).to_string();
            let ops = states.0.parse_line(&str, &SYNTAXSET);
            let h: Vec<_> = if let Ok(ops) = ops {
                RangedHighlightIterator::new(&mut states.1, &ops, &str, &self.highlighter)
                    .map(|h| SpanStyle::new(h.0, h.2))
                    .collect()
            } else {
                Vec::new()
            };
            let h = StyledLine::new(h);

            // let h = if let Some(str) = rope.line(i).as_str() {
            //     let ops = states.0.parse_line(&str, &SYNTAXSET);
            //     let h: Vec<_> = if let Ok(ops) = ops {
            //         RangedHighlightIterator::new(&mut states.1, &ops, &str, &self.highlighter)
            //             .map(|h| SpanStyle::new(h.0, h.2))
            //             .collect()
            //     } else {
            //         Vec::new()
            //     };
            //     StyledLine::new(h)
            // } else {
            //     let str = rope.line(i).to_string();
            //     let ops = states.0.parse_line(&str, &SYNTAXSET);
            //     let h: Vec<_> = if let Ok(ops) = ops {
            //         RangedHighlightIterator::new(&mut states.1, &ops, &str, &self.highlighter)
            //             .map(|h| SpanStyle::new(h.0, h.2))
            //             .collect()
            //     } else {
            //         Vec::new()
            //     };
            //     StyledLine::new(h)
            // };
            if i & 0xF == 0xF {
                self.states.push(states.clone());
            }
            let mut hl = highlighted_line.lines.lock().unwrap();
            if i >= hl.len() {
                hl.push(h);
            } else {
                hl[i] = h;
            }
        }
    }
}
