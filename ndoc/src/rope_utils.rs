use std::borrow::Cow;

use ropey::{str_utils::byte_to_char_idx, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

/// Finds the previous grapheme boundary before the given char position.
pub fn prev_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the previous grapheme cluster boundary.
    loop {
        match gc.prev_boundary(chunk, chunk_byte_idx) {
            Ok(None) => return 0,
            Ok(Some(n)) => {
                let tmp = byte_to_char_idx(chunk, n - chunk_byte_idx);
                return chunk_char_idx + tmp;
            }
            Err(GraphemeIncomplete::PrevChunk) => {
                let (a, b, c, _) = slice.chunk_at_byte(chunk_byte_idx - 1);
                chunk = a;
                chunk_byte_idx = b;
                chunk_char_idx = c;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx_chunk, n - ctx_chunk.len());
            }
            _ => unreachable!(),
        }
    }
}

/// Finds the next grapheme boundary after the given char position.
pub fn next_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the next grapheme cluster boundary.
    loop {
        match gc.next_boundary(chunk, chunk_byte_idx) {
            Ok(None) => return slice.len_chars(),
            Ok(Some(n)) => {
                let tmp = byte_to_char_idx(chunk, n - chunk_byte_idx);
                return chunk_char_idx + tmp;
            }
            Err(GraphemeIncomplete::NextChunk) => {
                chunk_byte_idx += chunk.len();
                let (a, _, c, _) = slice.chunk_at_byte(chunk_byte_idx);
                chunk = a;
                chunk_char_idx = c;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx_chunk, n - ctx_chunk.len());
            }
            _ => unreachable!(),
        }
    }
}

/// Finds the next grapheme boundary after the given char position.
pub fn next_grapheme_boundary_byte<U: Into<usize>>(slice: &RopeSlice, byte_idx: U) -> usize {
    let byte_idx = byte_idx.into();
    // Bounds check
    debug_assert!(byte_idx <= slice.len_bytes());

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, _, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the next grapheme cluster boundary.
    loop {
        match gc.next_boundary(chunk, chunk_byte_idx) {
            Ok(None) => return slice.len_bytes(),
            Ok(Some(n)) => {
                let tmp = n - chunk_byte_idx;
                return chunk_byte_idx + tmp;
            }
            Err(GraphemeIncomplete::NextChunk) => {
                chunk_byte_idx += chunk.len();
                let (a, b, _, _) = slice.chunk_at_byte(chunk_byte_idx);
                chunk = a;
                chunk_byte_idx = b;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx_chunk, n - ctx_chunk.len());
            }
            _ => unreachable!(),
        }
    }
}

/// Returns whether the given char position is a grapheme boundary.
pub fn is_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> bool {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (chunk, chunk_byte_idx, _, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Determine if the given position is a grapheme cluster boundary.
    loop {
        match gc.is_boundary(chunk, chunk_byte_idx) {
            Ok(n) => return n,
            Err(GraphemeIncomplete::PreContext(n)) => {
                let (ctx_chunk, ctx_byte_start, _, _) = slice.chunk_at_byte(n - 1);
                gc.provide_context(ctx_chunk, ctx_byte_start);
            }
            _ => unreachable!(),
        }
    }
}

const WORD_BOUNDARY_PUCTUATION: [char; 31] = [
    '`', '~', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '=', '+', '[', '{', ']', '}',
    '\\', '|', ';', ':', '\'', '"', ',', '.', '<', '>', '/', '?',
];
const WORD_BOUNDARY_LINEFEED: [char; 2] = ['\n', '\r'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharType {
    LineFeed,
    Space,
    Punctuation,
    Other,
}

fn char_type(c: char) -> CharType {
    if WORD_BOUNDARY_PUCTUATION.contains(&c) {
        CharType::Punctuation
    } else if WORD_BOUNDARY_LINEFEED.contains(&c) {
        CharType::LineFeed
    } else if c.is_whitespace() {
        CharType::Space
    } else {
        CharType::Other
    }
}

fn is_boundary(a: char, b: char) -> bool {
    char_type(a) != char_type(b)
}

pub fn next_word_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    let mut i = char_idx;

    // discard all space
    i += slice.chars_at(i).take_while(|c| c.is_whitespace()).count();

    // if multi puctionation, skip to new non puctuation char
    let fp = slice
        .chars_at(i)
        .take_while(|c| WORD_BOUNDARY_PUCTUATION.contains(c))
        .count();
    i += fp;
    if i >= slice.len_chars() {
        return slice.len_chars();
    }
    let current_char = slice.char(i);
    if fp > 1 || (fp == 1 && char_type(current_char) != CharType::Other) {
        return i;
    }

    i += slice
        .chars_at(i)
        .take_while(|c| !is_boundary(*c, current_char))
        .count();

    i
}

pub fn prev_word_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    let mut i = char_idx;
    // discard all space
    let mut iter = slice.chars_at(i);
    let mut count = 0;
    i -= loop {
        match iter.prev() {
            Some(c) if c.is_whitespace() => count += 1,
            _ => break count,
        }
    };

    // if multi puctionation, skip to new non puctuation char
    let mut iter = slice.chars_at(i);
    let mut count = 0;
    let fp = loop {
        match iter.prev() {
            Some(c) if WORD_BOUNDARY_PUCTUATION.contains(&c) => count += 1,
            _ => break count,
        }
    };
    i -= fp;
    if i == 0 {
        return 0;
    }

    let current_char = slice.char(i - 1);
    if fp > 1 || (fp == 1 && char_type(current_char) != CharType::Other) {
        return i;
    }

    let mut iter = slice.chars_at(i);
    let mut count = 0;
    i -= loop {
        match iter.prev() {
            Some(c) if !is_boundary(c, current_char) => count += 1,
            _ => break count,
        }
    };

    i
}

pub fn word_end(slice: &RopeSlice, char_idx: usize) -> usize {
    if char_idx >= slice.len_chars() {
        return slice.len_chars()
    }
    let mut i: usize = char_idx.into();
    let current_char = slice.char(i);
    i += slice.chars_at(i).take_while(|c| !is_boundary(*c, current_char)).count();
    i
}

pub fn word_start(slice: &RopeSlice, char_idx: usize) -> usize {
    if char_idx >= slice.len_chars() {
        return slice.len_chars()
    }
    let mut i: usize = char_idx;
    let current_char = slice.char(i);
    let mut iter = slice.chars_at(i);
    let mut count = 0;
    i -= loop {
        match iter.prev() {
            Some(c) if !is_boundary(c, current_char) => count += 1,
            _ => break count,
        }
    };
    i
}

pub fn get_line_start_boundary(slice: &RopeSlice, line_idx: usize) -> usize {
    slice
        .line(line_idx)
        .chars()
        .take_while(|c| c.is_whitespace())
        .count()
}

fn line_has_tab(rope: &RopeSlice, line_idx: usize) -> bool {
    for c in rope.line(line_idx).chunks() {
        if c.contains('\t') {
            return true;
        }
    }
    false
}

pub fn get_line_info<'a>(rope: &'a RopeSlice, line_idx: usize, indent_len: usize) -> Cow<'a, str> {
    if line_has_tab(rope, line_idx) {
        let mut s = String::with_capacity(rope.line(line_idx).len_chars());
        let mut offset = 0;
        for c in rope.line(line_idx).chars() {
            match c {
                '\t' => {
                    let tablen = indent_len - (offset % indent_len);
                    s.push_str(&" ".repeat(tablen));
                    offset += tablen;
                }
                _ => {
                    s.push(c);
                    offset += 1;
                }
            }
        }

        s.into()
    } else {
        match rope.line(line_idx).as_str() {
            Some(s) => s.into(),
            None => rope.line(line_idx).to_string().into(),
        }
    }
}

pub fn tab2space_char_idx(rope: &RopeSlice, line_idx: usize, indent_len: usize) -> Vec<usize> {
    let mut offset = 0;
    let mut v = Vec::with_capacity(rope.line(line_idx).len_chars());
    v.push(0);
    for c in rope.line(line_idx).chars() {
        match c {
            '\t' => {
                let tablen = indent_len - (offset % indent_len);
                offset += tablen;
            }
            _ => {
                offset += 1;
            }
        }
        v.push(offset);
    }
    v
}

pub fn grapheme_to_byte(slice: &RopeSlice, grapheme_idx: usize) -> usize {
    slice.char_to_byte(grapheme_to_char(slice,grapheme_idx))
}
pub fn grapheme_to_char(slice: &RopeSlice, grapheme_idx: usize) -> usize {
    let mut idx= 0;
    for count in 0 .. grapheme_idx {
        if count >= grapheme_idx {
            break;
        }
        idx = next_grapheme_boundary(slice, idx)
    }
    idx
}

pub fn byte_to_grapheme(slice: &RopeSlice, byte_idx: usize) -> usize {
    return char_to_grapheme(slice,slice.byte_to_char(byte_idx))
}

pub fn char_to_grapheme(slice: &RopeSlice, char_idx: usize) -> usize {
    let mut idx= 0;
    let mut count = 0;
    //let char_idx = slice.char_to_byte(char_idx);
    while idx<char_idx {
        idx = next_grapheme_boundary(slice, idx);
        count += 1;
    }
    count
}

pub struct NextGraphemeIdxIterator<'slice> {
    slice: &'slice RopeSlice<'slice>,
    index: Option<usize>,
}

impl<'slice> NextGraphemeIdxIterator<'slice> {
    pub fn new(slice: &'slice RopeSlice) -> Self {
        Self { slice, index: None }
    }
}

impl<'slice> Iterator for NextGraphemeIdxIterator<'slice> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(old_idx) = self.index {
            let idx = next_grapheme_boundary(&self.slice, old_idx);
            if idx == old_idx {
                None
            } else {
                self.index = Some(idx);
                Some(idx)//self.slice.char_to_byte(idx))
            }
        } else {
            self.index = Some(0);
            Some(0)
        }
    }
}

mod test {
    use std::borrow::Borrow;

    use ropey::Rope;

    use crate::rope_utils::get_line_info;

    #[test]
    fn file_info() {
        let rope = Rope::from_str("\txxxx\txx\tx\txxx");
        assert_eq!(
            get_line_info(&rope.slice(..), 0, 4),
            "    xxxx    xx  x   xxx"
        );
    }
}
