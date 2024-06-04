use std::fmt::Display;

use encoding_rs::Encoding;
use ropey::RopeSlice;
use serde::{Deserialize, Serialize};
use syntect::parsing::SyntaxReference;

use crate::syntax::SYNTAXSET;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub encoding: &'static Encoding,
    pub bom: Option<Vec<u8>>,
    pub linefeed: LineFeed,
    pub indentation: Indentation,
    pub syntax: &'static SyntaxReference
}

impl PartialEq for FileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.encoding == other.encoding && self.bom == other.bom && self.linefeed == other.linefeed && self.indentation == other.indentation && self.syntax.name == other.syntax.name
    }
}

impl Default for FileInfo {
    fn default() -> Self {
        Self {
            encoding: encoding_rs::UTF_8,
            bom: None,
            linefeed: Default::default(),
            indentation: Indentation::Tab(4),
            syntax: SYNTAXSET.find_syntax_plain_text(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFeed {
    CR,
    LF,
    CRLF,
}

impl Default for LineFeed {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        return LineFeed::CRLF;
        #[cfg(not(target_os = "windows"))]
        return LineFeed::LF;
    }
}

impl Display for LineFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineFeed::LF => write!(f, "\n"),
            LineFeed::CRLF => write!(f, "\r\n"),
            LineFeed::CR => write!(f, "\r"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Indentation {
    Tab(usize),
    Space(usize),
}

impl Default for Indentation {
    fn default() -> Self {
        Indentation::Space(4)
    }
}

impl Indentation {
    pub fn len(&self) -> usize {
        match self {
            Self::Tab(i) => *i,
            Self::Space(i) => *i,
        }
    }
}

impl ToString for Indentation {
    fn to_string(&self) -> String {
        match *self {
            Indentation::Tab(len) => format!("Tab ({len})"),
            Indentation::Space(len) => format!("Space ({len})"),
        }
    }
}

/// Detect the carriage return type of the buffer
pub fn detect_linefeed(input: &RopeSlice) -> LineFeed {
    let linefeed = Default::default();

    if input.len_bytes() == 0 {
        return linefeed;
    }

    let mut cr = 0;
    let mut lf = 0;
    let mut crlf = 0;

    let mut chars = input.chars().take(1000);
    while let Some(c) = chars.next() {
        if c == '\r' {
            if let Some(c2) = chars.next() {
                if c2 == '\n' {
                    crlf += 1;
                } else {
                    cr += 1;
                }
            }
        } else if c == '\n' {
            lf += 1;
        }
    }

    if cr > crlf && cr > lf {
        LineFeed::CR
    } else if lf > crlf && lf > cr {
        LineFeed::LF
    } else {
        LineFeed::CRLF
    }
}

pub fn detect_indentation(input: &RopeSlice) -> Indentation {
    // detect Tabs first. If the first char of a line is more often a Tab
    // then we consider the indentation as tabulation.

    let mut tab = 0;
    let mut space = 0;
    for line in input.lines() {
        match line.chars().next() {
            Some(' ') => space += 1,
            Some('\t') => tab += 1,
            _ => (),
        }
    }
    if tab > space {
        // todo: get len from settings
        return Indentation::Tab(4);
    }

    // Algorythm from
    // https://medium.com/firefox-developer-tools/detecting-code-indentation-eff3ed0fb56b
    use std::collections::HashMap;
    let mut indents = HashMap::new();
    let mut last = 0;

    for line in input.lines() {
        let width = line.chars().take_while(|c| *c == ' ').count();
        let indent = if width < last {
            last - width
        } else {
            width - last
        };
        if indent > 1 {
            (*indents.entry(indent).or_insert(0)) += 1;
        }
        last = width;
    }
    if let Some(i) = indents.iter().max_by(|x, y| x.1.cmp(y.1)) {
        Indentation::Space(*i.0)
    } else {
        Indentation::Space(4)
    }
}
