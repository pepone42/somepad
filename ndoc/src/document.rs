use std::{borrow::Cow, fs, io::Read, io::Result, path::Path};

use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use ropey::{Rope, RopeSlice};

use crate::{
    file_info::{detect_indentation, detect_linefeed, FileInfo, Indentation, LineFeed},
    rope_utils::{
        get_line_start_boundary, next_grapheme_boundary, next_word_boundary,
        prev_grapheme_boundary, prev_word_boundary,
    },
};

#[derive(Debug)]
pub struct Document {
    pub rope: Rope,
    edit_stack: Vec<(Rope, Vec<Selection>)>,
    edit_stack_top: usize,
    pub file_info: FileInfo,
    pub selections: Vec<Selection>,
}

impl Default for Document {
    fn default() -> Self {
        let rope = Rope::new();
        Self {
            rope: rope.clone(),
            edit_stack: vec![(rope, vec![Selection::default()])],
            edit_stack_top: 0,
            file_info: Default::default(),
            selections: vec![Selection::default()],
        }
    }
}

impl Document {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(&path)?;

        let mut detector = EncodingDetector::new();
        let mut vec = Vec::new();
        file.read_to_end(&mut vec)?;

        detector.feed(&vec, true);
        let encoding = Encoding::for_bom(&vec);

        match encoding {
            None => {
                let encoding = detector.guess(None, true);

                let rope = Rope::from_str(&encoding.decode_with_bom_removal(&vec).0);
                let linefeed = detect_linefeed(&rope.slice(..));
                let indentation = detect_indentation(&rope.slice(..));

                Ok(Self {
                    rope: rope.clone(),
                    edit_stack: vec![(rope, vec![Selection::default()])],
                    edit_stack_top: 0,
                    file_info: FileInfo {
                        encoding,
                        bom: None,
                        linefeed,
                        indentation,
                        syntax: "txt".to_owned(),
                    },
                    selections: vec![Selection::default()],
                })
            }
            Some((encoding, bom_size)) => {
                let bom = {
                    let mut v = Vec::new();
                    v.extend_from_slice(&vec[0..bom_size]);
                    v
                };
                let rope = Rope::from_str(&encoding.decode_with_bom_removal(&vec).0);
                let linefeed = detect_linefeed(&rope.slice(..));
                let indentation = detect_indentation(&rope.slice(..));

                Ok(Self {
                    rope: rope.clone(),
                    edit_stack: vec![(rope, vec![Selection::default()])],
                    edit_stack_top: 0,
                    file_info: FileInfo {
                        encoding,
                        bom: Some(bom),
                        linefeed,
                        indentation,
                        syntax: "txt".to_owned(),
                    },
                    selections: vec![Selection::default()],
                })
            }
        }
    }

    pub fn insert_at(&mut self, input: &str, start: usize, end: usize) {
        let mut changed = false;

        if start != end {
            let sel_idx = self
                .selections
                .iter()
                .map(|s| {
                    (
                        s.head.char_idx(&self.rope.slice(..)),
                        s.tail.char_idx(&self.rope.slice(..)),
                    )
                })
                .collect::<Vec<(usize, usize)>>();

            self.rope.remove(start..end);
            let to_sub = end - start;
            for i in 0..self.selections.len() {
                if sel_idx[i].0 >= end {
                    self.selections[i].head =
                        Position::from_char_idx(&self.rope.slice(..), sel_idx[i].0 - to_sub);
                }
                if sel_idx[i].1 >= end {
                    self.selections[i].tail =
                        Position::from_char_idx(&self.rope.slice(..), sel_idx[i].1 - to_sub);
                }
            }
            changed = true;
        }

        if input.len() > 0 {
            let sel_idx = self
                .selections
                .iter()
                .map(|s| {
                    (
                        s.head.char_idx(&self.rope.slice(..)),
                        s.tail.char_idx(&self.rope.slice(..)),
                    )
                })
                .collect::<Vec<(usize, usize)>>();
            self.rope.insert(start, input);

            // update selections after the insertion point
            let to_add = input.chars().count();
            for i in 0..self.selections.len() {
                if sel_idx[i].0 >= start {
                    self.selections[i].head =
                        Position::from_char_idx(&self.rope.slice(..), sel_idx[i].0 + to_add);
                }
                if sel_idx[i].1 >= start {
                    self.selections[i].tail =
                        Position::from_char_idx(&self.rope.slice(..), sel_idx[i].1 + to_add);
                }
            }
            changed = true;
        }

        if changed {
            self.edit_stack.drain(self.edit_stack_top + 1..);
            self.edit_stack
                .push((self.rope.clone(), self.selections.clone()));
            self.edit_stack_top += 1;
        }
    }

    pub fn undo(&mut self) {
        if self.edit_stack_top > 0 {
            self.edit_stack_top -= 1;
            (self.rope, self.selections) = self.edit_stack[self.edit_stack_top].clone();
        }
    }
    pub fn redo(&mut self) {
        if self.edit_stack_top < self.edit_stack.len() - 1 {
            self.edit_stack_top += 1;
            (self.rope, self.selections) = self.edit_stack[self.edit_stack_top].clone();
        }
    }

    pub fn insert(&mut self, input: &str) {
        for i in 0..self.selections.len() {
            let start = position_to_char(&self.rope.slice(..), self.selections[i].start());
            let end = position_to_char(&self.rope.slice(..), self.selections[i].end());

            self.selections[i].head.vcol = self.selections[i].head.column;
            self.selections[i].tail = self.selections[i].head;

            self.insert_at(input, start, end);
        }
        self.merge_selections();
    }

    pub fn backspace(&mut self) {
        for i in 0..self.selections.len() {
            if self.selections[i].head == self.selections[i].tail {
                let start = position_to_char(&self.rope.slice(..), self.selections[i].start());

                self.insert_at(
                    "",
                    prev_grapheme_boundary(&self.rope.slice(..), start),
                    start,
                );
            } else {
                let start = position_to_char(&self.rope.slice(..), self.selections[i].start());
                let end = position_to_char(&self.rope.slice(..), self.selections[i].end());
                self.insert_at("", start, end);
            }
            self.selections[i].head.vcol = self.selections[i].head.column;
            self.selections[i].tail = self.selections[i].head;
        }
        self.merge_selections();
    }

    pub fn delete(&mut self) {
        for i in 0..self.selections.len() {
            if self.selections[i].head == self.selections[i].tail {
                let start = position_to_char(&self.rope.slice(..), self.selections[i].start());

                self.insert_at(
                    "",
                    start,
                    next_grapheme_boundary(&self.rope.slice(..), start),
                );
            } else {
                let start = position_to_char(&self.rope.slice(..), self.selections[i].start());
                let end = position_to_char(&self.rope.slice(..), self.selections[i].end());
                self.insert_at("", start, end);
            }
            self.selections[i].head.vcol = self.selections[i].head.column;
            self.selections[i].tail = self.selections[i].head;
        }
        self.merge_selections();
    }

    pub fn move_selections(&mut self, dir: MoveDirection, expand: bool) {
        for s in &mut self.selections {
            match dir {
                MoveDirection::Up => {
                    s.head.line = s.head.line.saturating_sub(1);
                    s.head.column = s.head.vcol.min(line_len_char(&self.rope, s.head.line));
                }
                MoveDirection::Down => {
                    s.head.line = usize::min(s.head.line + 1, self.rope.len_lines() - 1);
                    s.head.column = s.head.vcol.min(line_len_char(&self.rope, s.head.line));
                }
                MoveDirection::Left => {
                    let start = s.head.char_idx(&self.rope.slice(..));
                    s.head = Position::from_char_idx(
                        &self.rope.slice(..),
                        prev_grapheme_boundary(&self.rope.slice(..), start),
                    );
                }
                MoveDirection::Right => {
                    let start = s.head.char_idx(&self.rope.slice(..));
                    s.head = Position::from_char_idx(
                        &self.rope.slice(..),
                        next_grapheme_boundary(&self.rope.slice(..), start),
                    );
                }
            }
            if !expand {
                s.tail = s.head;
            }
        }
        self.merge_selections();
    }

    pub fn move_selections_word(&mut self, dir: MoveDirection, expand: bool) {
        for s in &mut self.selections {
            match dir {
                MoveDirection::Left => {
                    let start = s.head.char_idx(&self.rope.slice(..));
                    s.head = Position::from_char_idx(
                        &self.rope.slice(..),
                        prev_word_boundary(&self.rope.slice(..), start),
                    );
                }
                MoveDirection::Right => {
                    let start = s.head.char_idx(&self.rope.slice(..));
                    s.head = Position::from_char_idx(
                        &self.rope.slice(..),
                        next_word_boundary(&self.rope.slice(..), start),
                    );
                }
                _ => (),
            }
            if !expand {
                s.tail = s.head;
            }
        }
        self.merge_selections();
    }

    pub fn duplicate_selection(&mut self, direction: MoveDirection) {
        match direction {
            MoveDirection::Down => {
                let s = *self.selections.iter().max().unwrap();
                let mut news = s;
                news.head.line = usize::min(s.head.line + 1, self.rope.len_lines() - 1);
                news.head.column = s.head.vcol.min(line_len_char(&self.rope, news.head.line));
                news.tail = news.head;
                if news.head.line > s.head.line {
                    self.selections.push(news);
                }
            }
            MoveDirection::Up => {
                let s = *self.selections.iter().min().unwrap();
                let mut news = s;
                news.head.line = s.head.line.saturating_sub(1);
                news.head.column = s.head.vcol.min(line_len_char(&self.rope, news.head.line));
                news.tail = news.head;
                if news.head.line < s.head.line {
                    self.selections.push(news);
                }
            }
            _ => (),
        }

        self.merge_selections();
    }

    pub fn page_up(&mut self, amount: usize, expand: bool) {
        for s in &mut self.selections {
            s.head.line = s.head.line.saturating_sub(amount);
            s.head.column = s.head.vcol.min(line_len_char(&self.rope, s.head.line));
            if !expand {
                s.tail = s.head;
            }
        }
        self.merge_selections();
    }

    pub fn page_down(&mut self, amount: usize, expand: bool) {
        for s in &mut self.selections {
            s.head.line = usize::min(s.head.line + amount, self.rope.len_lines() - 1);
            s.head.column = s.head.vcol.min(line_len_char(&self.rope, s.head.line));
            if !expand {
                s.tail = s.head;
            }
        }
        self.merge_selections();
    }

    pub fn home(&mut self, expand: bool) {
        for s in &mut self.selections {
            s.head.column = match s.head.column {
                c if c == get_line_start_boundary(&self.rope.slice(..), s.head.line) => 0,
                _ => get_line_start_boundary(&self.rope.slice(..), s.head.line),
            };
            if !expand {
                s.tail = s.head;
            }
        }

        self.merge_selections();
    }
    pub fn end(&mut self, expand: bool) {
        for s in &mut self.selections {
            s.head.column = line_len_char(&self.rope, s.head.line);
            if !expand {
                s.tail = s.head;
            }
        }

        self.merge_selections();
    }

    pub fn indent(&mut self, always: bool) {
        let main_sel = self.selections.first().unwrap();
        if always || main_sel.head.line != main_sel.tail.line {
            for s in self.selections.clone() {
                for l in s.start().line..=s.end().line {
                    let index = self.rope.line_to_char(l);
                    match self.file_info.indentation {
                        Indentation::Tab(_) => self.insert_at("\t", index, index),
                        Indentation::Space(x) => self.insert_at(&" ".repeat(x), index, index),
                    }
                }
            }
        } else {
            for s in self.selections.clone() {
                let index = s.head.char_idx(&self.rope.slice(..));
                match self.file_info.indentation {
                    Indentation::Tab(_) => self.insert_at("\t", index, index),
                    Indentation::Space(x) => {
                        let repeat = x - (s.head.column % x);
                        self.insert_at(&" ".repeat(repeat), index, index);
                    }
                }
            }
        }
    }

    pub fn deindent(&mut self) {
        for s in self.selections.clone() {
            for l in s.start().line..=s.end().line {
                let index = self.rope.line_to_char(l);

                let line_start = get_line_start_boundary(&self.rope.slice(..), l);
                match self.file_info.indentation {
                    Indentation::Tab(_) => self.insert_at("", index, index + 1),
                    Indentation::Space(x) => {
                        let r = line_start.min(x);
                        self.insert_at("", index, index + r);
                    }
                }
            }
        }
    }

    fn merge_selections(&mut self) {
        if self.selections.len() == 1 {
            return;
        }
        self.selections
            .sort_unstable_by(|a, b| a.start().cmp(&b.start()));
        let mut redo = true;
        'outer: while redo {
            for i in 0..self.selections.len() - 1 {
                if self.selections[i].collide_with(self.selections[i + 1]) {
                    let s = self.selections[i + 1];
                    self.selections[i].merge_with(s);
                    self.selections.remove(i + 1);
                    redo = true;
                    continue 'outer;
                }
            }
            redo = false;
        }
    }
}

pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default, Debug, Clone, Copy, Eq, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    vcol: usize,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line && self.column == other.column
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.line.partial_cmp(&other.line) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.vcol.partial_cmp(&other.column)
    }
}

impl Position {
    pub fn new(line_idx: usize, column_idx: usize) -> Self {
        Self {
            line: line_idx,
            column: column_idx,
            vcol: column_idx,
        }
    }

    pub fn from_char_idx(rope: &RopeSlice, char_idx: usize) -> Self {
        char_to_position(&rope, char_idx)
    }

    pub fn char_idx(&self, rope: &RopeSlice) -> usize {
        position_to_char(&rope, *self)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Ord)]
pub struct Selection {
    pub head: Position,
    pub tail: Position,
}

impl PartialOrd for Selection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.tail.partial_cmp(&other.head)
    }
}

impl Selection {
    pub fn start(&self) -> Position {
        if self.head <= self.tail {
            self.head
        } else {
            self.tail
        }
    }
    pub fn end(&self) -> Position {
        if self.head > self.tail {
            self.head
        } else {
            self.tail
        }
    }
    pub fn areas(&self, rope: &Rope) -> Vec<(usize, usize)> {
        match self.end().line - self.start().line {
            0 => {
                vec![(self.start().column, self.end().column)]
            }
            1 => {
                vec![
                    (
                        self.start().column,
                        line_len_char(rope, self.start().line),
                    ),
                    (0, self.end().column),
                ]
            }
            _ => {
                let mut v = Vec::new();
                v.push((
                    self.start().column,
                    line_len_char(rope, self.start().line),
                ));

                for l in self.start().line + 1..self.end().line {
                    v.push((0, line_len_char(rope, l)));
                }

                v.push((0, self.end().column));
                v
            }
        }
    }

    pub fn merge_with(&mut self, other: Selection) -> bool {
        if self.head < self.tail {
            if other.head < self.tail {
                self.tail = other.tail.max(self.tail);
                return true;
            }
        } else {
            if other.tail < self.head {
                self.head = other.head.max(self.head);
                return true;
            }
        }
        false
    }

    pub fn collide_with(&self, other: Selection) -> bool {
        self.end() > other.start() || (self.head == self.tail && self.head == other.head)
    }
}

// TODO: Unoptimal
pub fn line_len_char(rope: &Rope, line_idx: usize) -> usize {
    let mut r = rope.line(line_idx).chars().collect::<Vec<char>>();
    r.reverse();
    let linefeed_len = match (r.get(1), r.get(0)) {
        (Some('\u{000D}'), Some('\u{000A}')) => 2,
        (_, Some('\u{000A}')) => 1,
        (_, Some('\u{000D}')) => 1,
        (_, _) => 0,
    };
    r.len() - linefeed_len
}

pub fn position_to_char(rope: &RopeSlice, position: Position) -> usize {
    let l = rope.line_to_char(position.line);
    l + position.column
}

#[test]
fn test_position_to_char() {
    let r = Rope::from("aaaa\r\n\r\n\r\naaaa\r\n");
    dbg!(r.line_to_char(3));
    assert_eq!(position_to_char(&r.slice(..), Position::new(3, 0)), 10);
}

pub fn char_to_position(rope: &RopeSlice, char_idx: usize) -> Position {
    let line = rope.char_to_line(char_idx.min(rope.len_chars()));
    let column = char_idx - rope.line_to_char(line);
    Position::new(line, column)
}

pub fn line_len_grapheme(rope: &Rope, line_idx: usize) -> usize {
    let line = rope.line(line_idx);
    let mut idx = 0;
    let mut last_idx = 0;
    let mut grapheme_count = 0;
    while idx < line.len_chars() {
        last_idx = idx;
        idx = next_grapheme_boundary(&line, idx);
        grapheme_count += 1;
    }
    let last = line.slice(last_idx..idx).to_string();
    grapheme_count
        - match last.as_str() {
            "\r\n" | "\n" | "\r" => 1,
            _ => 0,
        }
}


