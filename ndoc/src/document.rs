use std::{
    borrow::Cow,
    fs,
    io::Read,
    io::Result,
    path::{Path, PathBuf},
};

use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use ropey::{Rope, RopeBuilder, RopeSlice};

#[cfg(feature = "vizia")]
use vizia::prelude::*;

use crate::{
    file_info::{detect_indentation, detect_linefeed, FileInfo, Indentation, LineFeed},
    rope_utils::{
        char_to_grapheme, get_line_start_boundary, grapheme_to_byte, grapheme_to_char,
        next_grapheme_boundary, next_word_boundary, prev_grapheme_boundary, prev_word_boundary,
        word_end, word_start,
    },
};

#[derive(Debug, Clone)]
pub struct Document {
    pub rope: Rope,
    edit_stack: Vec<(Rope, Vec<Selection>)>,
    edit_stack_top: usize,
    pub file_info: FileInfo,
    pub selections: Vec<Selection>,
    pub file_name: Option<PathBuf>,
}

#[cfg(feature = "vizia")]
impl Data for Document {
    fn same(&self, other: &Self) -> bool {
        *self == *other
    }
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.rope == other.rope
            && self.edit_stack == other.edit_stack
            && self.edit_stack_top == other.edit_stack_top
            && self.file_info == other.file_info
            && self.selections == other.selections
    }
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
            file_name: None,
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
                    file_name: Some(path.as_ref().to_path_buf()),
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
                    file_name: Some(path.as_ref().to_path_buf()),
                })
            }
        }
    }

    pub fn insert_at_position(&mut self, input: &str, start: Position, end: Position) {
        let start = self.position_to_char(start);
        let end = self.position_to_char(end);
        self.insert_at(input, start, end);
    }

    pub fn insert_at_selection(&mut self, input: &str, selection: Selection) {
        self.insert_at_position(input, selection.start(), selection.end());
    }

    pub fn insert_at(&mut self, input: &str, start: usize, end: usize) {
        let mut changed = false;

        if start != end {
            let sel_idx = self
                .selections
                .iter()
                .map(|s| {
                    (
                        position_to_char(&self.rope.slice(..), s.head),
                        position_to_char(&self.rope.slice(..), s.tail),
                    )
                })
                .collect::<Vec<(usize, usize)>>();

            self.rope.remove(start..end);
            let to_sub = end - start;
            for i in 0..self.selections.len() {
                if sel_idx[i].0 >= end {
                    self.selections[i].head =
                        char_to_position(&self.rope.slice(..), sel_idx[i].0 - to_sub);
                }
                if sel_idx[i].1 >= end {
                    self.selections[i].tail =
                        char_to_position(&self.rope.slice(..), sel_idx[i].1 - to_sub);
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
                        position_to_char(&self.rope.slice(..), s.head),
                        position_to_char(&self.rope.slice(..), s.tail),
                    )
                })
                .collect::<Vec<(usize, usize)>>();
            self.rope.insert(start, input);

            // update selections after the insertion point
            let to_add = input.chars().count();
            for i in 0..self.selections.len() {
                if sel_idx[i].0 >= start {
                    self.selections[i].head =
                        char_to_position(&self.rope.slice(..), sel_idx[i].0 + to_add);
                }
                if sel_idx[i].1 >= start {
                    self.selections[i].tail =
                        char_to_position(&self.rope.slice(..), sel_idx[i].1 + to_add);
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

    pub fn position_to_char(&self, position: Position) -> usize {
        position_to_char(&self.rope.slice(..), position)
    }

    pub fn char_to_position(&self, char_idx: usize) -> Position {
        char_to_position(&self.rope.slice(..), char_idx)
    }

    pub fn get_selection_content(&self) -> String {
        let r = self
            .selections
            .iter()
            .map(|s| {
                self.rope
                    .slice(self.position_to_char(s.start())..self.position_to_char(s.end()))
                    .to_string()
            })
            .collect::<Vec<String>>();
        r.join(&LineFeed::default().to_string())
    }

    pub fn insert_many(&mut self, input: &str) {
        if self.selections.len() > 1 && input.lines().count() == self.selections.len() {
            for (i, l) in input.lines().enumerate() {
                self.insert_at_selection(l, self.selections[i]);
            }
        } else {
            self.insert(input);
        }
    }

    pub fn insert(&mut self, input: &str) {
        for i in 0..self.selections.len() {
            self.insert_at_selection(input, self.selections[i]);
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
                    s.head.column = s
                        .head
                        .vcol
                        .min(line_len_grapheme(&self.rope.slice(..), s.head.line));
                }
                MoveDirection::Down => {
                    s.head.line = usize::min(s.head.line + 1, self.rope.len_lines() - 1);
                    s.head.column = s
                        .head
                        .vcol
                        .min(line_len_grapheme(&self.rope.slice(..), s.head.line));
                }
                MoveDirection::Left => {
                    let start = position_to_char(&self.rope.slice(..), s.head);
                    s.head = char_to_position(
                        &self.rope.slice(..),
                        prev_grapheme_boundary(&self.rope.slice(..), start),
                    );
                }
                MoveDirection::Right => {
                    let start = position_to_char(&self.rope.slice(..), s.head);
                    s.head = char_to_position(
                        &self.rope.slice(..),
                        next_grapheme_boundary(&self.rope.slice(..), start),
                    );
                    s.head;
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
                    let start = position_to_char(&self.rope.slice(..), s.head);
                    s.head = char_to_position(
                        &self.rope.slice(..),
                        prev_word_boundary(&self.rope.slice(..), start),
                    );
                }
                MoveDirection::Right => {
                    let start = position_to_char(&self.rope.slice(..), s.head);
                    s.head = char_to_position(
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

    pub fn next_word_boundary(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(
            slice,
            next_word_boundary(slice, position_to_char(slice, position)),
        )
    }

    pub fn prev_word_boundary(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(
            slice,
            prev_word_boundary(slice, position_to_char(slice, position)),
        )
    }

    pub fn word_start(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(slice, word_start(slice, position_to_char(slice, position)))
    }

    pub fn word_end(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(slice, word_end(slice, position_to_char(slice, position)))
    }

    pub fn select_word(&mut self, position: Position) {
        let tail = self.word_start(position);
        let head = self.word_end(position);
        self.selections = vec![Selection { head, tail }]
    }

    pub fn expand_selection_by_word(&mut self, position: Position) {
        if position < self.selections[0].tail {
            let end = self.selections[0].end();
            self.selections[0].head = self.word_start(position);
            self.selections[0].tail = end;
        } else if position > self.selections[0].tail {
            let start = self.selections[0].start();
            self.selections[0].head = self.word_end(position);
            self.selections[0].tail = start;
        }
    }

    pub fn expand_selection_by_line(&mut self, position: Position) {
        if position < self.selections[0].tail {
            let end = self.selections[0].end();
            self.selections[0].head = self.line_start(position.line);
            self.selections[0].tail = end;
        } else if position > self.selections[0].tail {
            let start = self.selections[0].start();
            self.selections[0].head = self.line_end_full(position.line);
            self.selections[0].tail = start;
        }
    }

    pub fn line_start(&mut self, line: usize) -> Position {
        char_to_position(&self.rope.slice(..), self.rope.line_to_char(line))
    }

    pub fn line_end(&mut self, line: usize) -> Position {
        char_to_position(
            &self.rope.slice(..),
            self.rope.line_to_char(line) + line_len_char(&self.rope.slice(..), line),
        )
    }

    pub fn line_end_full(&mut self, line: usize) -> Position {
        self.line_start(line + 1)
    }

    pub fn select_line(&mut self, line: usize) {
        let tail = self.line_start(line);
        let head = self.line_end_full(line);
        self.selections = vec![Selection { head, tail }]
    }

    pub fn select_all(&mut self) {
        let tail = char_to_position(&self.rope.slice(..), 0);
        let head = char_to_position(&self.rope.slice(..), self.rope.len_chars());
        self.selections = vec![Selection { head, tail }]
    }

    pub fn duplicate_selection(&mut self, direction: MoveDirection) {
        match direction {
            MoveDirection::Down => {
                let s = *self.selections.iter().max().unwrap();
                let mut news = s;
                news.head.line = usize::min(s.head.line + 1, self.rope.len_lines() - 1);
                news.head.column = s
                    .head
                    .vcol
                    .min(line_len_grapheme(&self.rope.slice(..), news.head.line));
                news.tail = news.head;
                if news.head.line > s.head.line {
                    self.selections.push(news);
                }
            }
            MoveDirection::Up => {
                let s = *self.selections.iter().min().unwrap();
                let mut news = s;
                news.head.line = s.head.line.saturating_sub(1);
                news.head.column = s
                    .head
                    .vcol
                    .min(line_len_grapheme(&self.rope.slice(..), news.head.line));
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
            s.head.column = s
                .head
                .vcol
                .min(line_len_grapheme(&self.rope.slice(..), s.head.line));
            if !expand {
                s.tail = s.head;
            }
        }
        self.merge_selections();
    }

    pub fn page_down(&mut self, amount: usize, expand: bool) {
        for s in &mut self.selections {
            s.head.line = usize::min(s.head.line + amount, self.rope.len_lines() - 1);
            s.head.column = s
                .head
                .vcol
                .min(line_len_grapheme(&self.rope.slice(..), s.head.line));
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
            s.head.vcol = s.head.column;
            if !expand {
                s.tail = s.head;
            }
        }

        self.merge_selections();
    }
    pub fn end(&mut self, expand: bool) {
        for s in &mut self.selections {
            s.head.column = line_len_grapheme(&self.rope.slice(..), s.head.line);
            s.head.vcol = s.head.column;
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
                let index = position_to_char(&self.rope.slice(..), s.head);
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

#[derive(Default, Debug, Clone, Copy, Eq, Ord, Hash)]
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
        self.column.partial_cmp(&other.column)
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

    // pub fn from_char_idx(rope: &RopeSlice, char_idx: usize) -> Self {
    //     char_to_position(&rope, char_idx)
    // }

    // pub fn char_idx(&self, rope: &RopeSlice) -> usize {
    //     position_to_char(&rope, *self)
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionAera {
    pub col_start: usize,
    pub col_end: usize,
    pub line: usize,
    pub include_eol: bool,
    pub id: Selection,
}

impl SelectionAera {
    pub fn new(
        col_start: usize,
        col_end: usize,
        line: usize,
        include_eol: bool,
        id: Selection,
    ) -> Self {
        Self {
            col_start,
            col_end,
            line,
            include_eol,
            id,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Ord, Hash)]
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
    pub fn new(line: usize, col: usize) -> Self {
        Self {
            head: Position::new(line, col),
            tail: Position::new(line, col),
        }
    }
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
    pub fn areas(&self, rope: &Rope) -> Vec<SelectionAera> {
        match self.end().line - self.start().line {
            0 => {
                vec![SelectionAera::new(
                    self.start().column,
                    self.end().column,
                    self.start().line,
                    false,
                    *self,
                )]
            }
            1 => {
                vec![
                    SelectionAera::new(
                        self.start().column,
                        line_len_grapheme(&rope.slice(..), self.start().line),
                        self.start().line,
                        true,
                        *self,
                    ),
                    SelectionAera::new(0, self.end().column, self.end().line, false, *self),
                ]
            }
            _ => {
                let mut v = Vec::new();
                v.push(SelectionAera::new(
                    self.start().column,
                    line_len_grapheme(&rope.slice(..), self.start().line),
                    self.start().line,
                    true,
                    *self,
                ));

                for l in self.start().line + 1..self.end().line {
                    v.push(SelectionAera::new(
                        0,
                        line_len_grapheme(&rope.slice(..), l),
                        l,
                        true,
                        *self,
                    ));
                }

                v.push(SelectionAera::new(
                    0,
                    self.end().column,
                    self.end().line,
                    false,
                    *self,
                ));
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

    pub fn is_single_line(&self) -> bool {
        self.head.line == self.tail.line
    }
}

// TODO: Unoptimal
pub fn line_len_char(rope: &RopeSlice, line_idx: usize) -> usize {
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

pub fn line_len_char_full(rope: &RopeSlice, line_idx: usize) -> usize {
    rope.line_to_char(line_idx + 1) - rope.line_to_char(line_idx)
}

pub fn position_to_char(slice: &RopeSlice, position: Position) -> usize {
    let l = slice.line_to_char(position.line);
    l + grapheme_to_char(&slice.line(position.line), position.column)
}

#[test]
fn test_position_to_char() {
    let r = Rope::from("aaaa\r\n\r\n\r\naaaa\r\n");
    dbg!(r.line_to_char(3));
    assert_eq!(position_to_char(&r.slice(..), Position::new(3, 0)), 10);
}

#[test]
fn test_position_from_char_idx() {
    let source = include_str!("../test_assets/utf8-demo.txt");
    let expected = include_str!("../test_assets/expected.txt");
    let expected = expected
        .lines()
        .map(|l| l.split(' '))
        .map(|mut l| {
            //dbg!(l.nth(0),l.nth(0),l.nth(0));
            (
                l.nth(0).unwrap().parse::<usize>().unwrap(),
                l.nth(0).unwrap().parse::<usize>().unwrap(),
                l.nth(0).unwrap().parse::<usize>().unwrap(),
            )
        })
        .collect::<Vec<(usize, usize, usize)>>();
    let rope = Rope::from(source);
    let slice = rope.slice(..);

    for e in expected {
        //dbg!(e.0,e.1,e.2,slice.line(e.1).to_string());
        assert_eq!(
            char_to_position(&slice, rope.byte_to_char(e.0)),
            Position::new(e.1, e.2),
            "testing byte index {} (char {}) for line {}",
            e.0,
            rope.byte_to_char(e.0),
            slice.line(e.1).to_string()
        );
    }
}

// #[test]
// fn test_position_from_char_idx_point() {
//     let rope = Rope::from("  Misc Sm Circles:  ｡ ⋄ ° ﾟ ˚ ﹾ");
//     assert_eq!(char_to_position(&rope.slice(..), 27), Position::new(0, 27));
// }

#[test]
fn test_char_to_grapheme() {
    let rope = Rope::from("  Diamonds:         ⋄ ᛜ ⌔ ◇ ⟐ ◈ ◆   ◊");
    let column = char_to_grapheme(&rope.slice(..), 27);
    assert_eq!(column, 27);
}

pub fn char_to_position(rope: &RopeSlice, char_idx: usize) -> Position {
    let line = rope.char_to_line(char_idx.min(rope.len_chars()));
    //let column = print_positions::print_positions(&rope.line(line).chars().take(char_idx).collect::<String>()).count();
    let column = char_to_grapheme(&rope.line(line), char_idx - rope.line_to_char(line));
    Position::new(line, column)
}

pub fn line_len_grapheme(rope: &RopeSlice, line_idx: usize) -> usize {
    //line_len_char(rope, line_idx)
    char_to_grapheme(&rope.line(line_idx), line_len_char(rope, line_idx))
}

pub fn line_len_grapheme_full(rope: &RopeSlice, line_idx: usize) -> usize {
    //line_len_char(rope, line_idx)
    char_to_grapheme(&rope.line(line_idx), line_len_char_full(rope, line_idx))
}
