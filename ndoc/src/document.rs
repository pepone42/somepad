use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::format,
    fs,
    io::{Read, Result, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use itertools::Itertools;
use once_cell::sync::Lazy;
use ropey::{Rope, RopeSlice};
use syntect::parsing::SyntaxReference;

use crate::{
    file_info::{detect_indentation, detect_linefeed, FileInfo, Indentation, LineFeed},
    rope_utils::{
        self, char_to_grapheme, get_line_start_boundary, grapheme_to_char, next_grapheme_boundary,
        next_word_boundary, prev_grapheme_boundary, prev_word_boundary, word_end, word_start,
    },
    syntax::{StateCache, StyledLine, StyledLinesCache, SYNTAXSET},
};

static DOCID: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_SENDER: Lazy<Arc<Mutex<Option<Sender<BackgroundWorkerMessage>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Debug, Clone, Default)]
struct History {
    edit_stack: Vec<(Rope, Vec<Selection>)>,
    edit_stack_top: usize,
    last_action: Action,
}

impl History {
    fn is_empty(&self) -> bool {
        self.edit_stack_top == 0
    }

    fn push(&mut self, rope: Rope, selections: Vec<Selection>, action: &Action) {
        if self.should_push(action, &self.last_action) {
            self.edit_stack.drain(self.edit_stack_top..);
            self.edit_stack.push((rope, selections));
            self.edit_stack_top += 1;
        }
        self.last_action = action.clone();
    }

    fn undo(&mut self, rope: Rope, selections: Vec<Selection>) -> Option<(Rope, Vec<Selection>)> {
        if self.edit_stack_top == self.edit_stack.len() {
            self.edit_stack.push((rope, selections));
        }

        if self.edit_stack_top > 0 {
            self.edit_stack_top -= 1;
            Some(self.edit_stack[self.edit_stack_top].clone())
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<(Rope, Vec<Selection>)> {
        if self.edit_stack_top + 1 < self.edit_stack.len() {
            self.edit_stack_top += 1;
            Some(self.edit_stack[self.edit_stack_top].clone())
        } else {
            None
        }
    }

    fn should_push(&self, action: &Action, last_action: &Action) -> bool {
        if self.is_empty() {
            return true;
        }
        match (action, last_action) {
            (Action::Delete, Action::Delete) => false,
            (Action::Backspace, Action::Backspace) => false,
            (Action::Delete, _) => true,
            (Action::Backspace, _) => true,
            (Action::Tab, _) => true,
            (_, Action::Tab) => true,
            (Action::Text(t), _) if t.chars().count() > 1 => true,
            (Action::Text(t), _) if t.chars().nth(0).is_some_and(|c| !c.is_alphanumeric()) => true,
            (_, _) => false,
        }
    }
}

#[derive(Debug, Default, Clone)]
enum Action {
    #[default]
    None,
    Backspace,
    Delete,
    Text(String),
    Tab,
}

#[test]
fn new_doc_id() {
    DOCID.store(0, Ordering::Relaxed);
    let d1 = Document::default();
    let d2 = Document::default();
    assert_eq!(d1.id, 0);
    assert_eq!(d2.id, 1);
}

enum BackgroundWorkerMessage {
    Stop,
    RegisterDocument(usize, Box<dyn Send + Fn()>),
    UpdateBuffer(
        usize,
        SyntaxReference,
        Rope,
        usize,
        StyledLinesCache,
        Sender<()>,
        usize,
    ),
    UpdateTheme(String),
    // WatchFile(PathBuf),
    // UnwatchFile(PathBuf),
}

struct HighlighterState<'a> {
    doc_id: usize,
    syntax: &'a SyntaxReference,
    state_cache: StateCache,
    current_index: usize,
    chunk_len: usize,
    rope: Rope,
    lines_cache: StyledLinesCache,
    tab_len: usize,
}

impl<'a> HighlighterState<'a> {
    fn new(doc_id: usize) -> Self {
        Self {
            doc_id,
            syntax: SYNTAXSET.find_syntax_plain_text(),
            state_cache: StateCache::new(),
            current_index: 0,
            chunk_len: 100,
            rope: Rope::new(),
            lines_cache: StyledLinesCache::new(),
            tab_len: 4,
        }
    }

    fn update_chunk(&mut self) {
        self.state_cache.update_range(
            &self.lines_cache,
            self.syntax,
            &self.rope,
            self.current_index,
            self.current_index + self.chunk_len,
            self.tab_len,
        );
        self.current_index += self.chunk_len;
        // subsequent chunck are bigger, for better performance
        self.chunk_len = 1000;
    }

    fn update_theme(&mut self, theme: &str) {
        self.state_cache.change_theme(theme);
    }
}

#[derive(Debug, Clone)]
struct BatchEdit {
    rope: Rope,
    selections: Vec<Selection>,
    action: Action,
    from_char_idx: usize,
    have_change: bool,
}

/// A Document represent a editable text, it can be attached to a file or not
/// Document use Ropey as the underlying data structure
#[derive(Debug, Clone)]
pub struct Document {
    id: usize,
    /// The text content
    pub rope: Rope,
    history: History,
    /// File information about the document such as encoding, syntax, indentation, etc
    pub file_info: FileInfo,
    /// The selections (or cursors) used to edit the document
    pub selections: Vec<Selection>,
    /// The filename of the document if a file is bound to it, None otherwise
    pub file_name: Option<PathBuf>,
    message_sender: Option<Sender<BackgroundWorkerMessage>>,
    line_style_cache: StyledLinesCache,
    batch_edit: Option<BatchEdit>,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.rope == other.rope
            && self.file_info == other.file_info
            && self.selections == other.selections
            && self.file_name == other.file_name
    }
}

impl Default for Document {
    fn default() -> Self {
        let rope = Rope::new();
        let message_sender = if let Ok(mg) = MESSAGE_SENDER.lock() {
            mg.clone()
        } else {
            None
        };

        Self {
            id: Document::new_id(),
            rope: rope.clone(),
            selections: vec![Selection::default()],
            file_name: None,
            history: Default::default(),
            file_info: Default::default(),
            message_sender,
            line_style_cache: StyledLinesCache::new(),
            batch_edit: None,
        }
    }
}

impl Document {
    fn new_id() -> usize {
        DOCID.fetch_add(1, Ordering::Relaxed)
    }

    /// Create a new Document with the given indentation
    pub fn new(indentation: Indentation) -> Self {
        let mut d = Document::default();
        d.file_info.indentation = indentation;
        d
    }

    /// Return the id of the document
    pub fn id(&self) -> usize {
        self.id
    }

    /// Return the title of the document
    pub fn title(&self) -> String {
        if let Some(f) = &self.file_name {
            f.file_name().unwrap().to_string_lossy().into()
        } else {
            format!("Untitled {}", self.id())
        }
    }

    /// Initialize the highlighter thread
    pub fn init_highlighter() {
        if MESSAGE_SENDER.lock().is_ok_and(|m| m.is_some()) {
            return;
        }

        let (tx, rx) = mpsc::channel();

        (*MESSAGE_SENDER.lock().unwrap()) = Some(tx);

        let _ = thread::Builder::new()
            .name("Document Highlighter".to_string())
            .spawn(move || {
                let mut highlight_state = HashMap::new();
                let mut callback = HashMap::new();

                loop {
                    match rx.try_recv() {
                        Ok(BackgroundWorkerMessage::UpdateBuffer(
                            id,
                            s,
                            r,
                            start,
                            cache,
                            tx,
                            tab_len,
                        )) => {
                            let state = highlight_state
                                .entry(id)
                                .or_insert(HighlighterState::new(id));

                            state.rope = r;
                            state.tab_len = tab_len;
                            state.current_index = start;
                            state.syntax = SYNTAXSET.find_syntax_by_name(&s.name).unwrap();
                            state.lines_cache = cache;
                            // smaller chunk for the first synchronous update
                            state.chunk_len = 100;
                            state.update_chunk();
                            let _ = tx.send(());
                        }
                        Ok(BackgroundWorkerMessage::RegisterDocument(id, f)) => {
                            callback.insert(id, f);
                        }
                        Ok(BackgroundWorkerMessage::UpdateTheme(theme)) => {
                            for (_, state) in highlight_state.iter_mut() {
                                state.update_theme(&theme);
                            }
                        }
                        Ok(BackgroundWorkerMessage::Stop) => return,
                        _ => (),
                    }
                    if highlight_state
                        .values()
                        .any(|s| s.current_index < s.rope.len_lines())
                    {
                        for h in highlight_state.values_mut() {
                            h.update_chunk();
                        }
                        for id in highlight_state.keys() {
                            if let Some(f) = callback.get(id) {
                                f();
                            }
                        }
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            });
    }

    /// Register a callback to be called when the highlighter update the document
    pub fn on_highlighter_update(&self, f: impl Fn() + Send + 'static) {
        if let Some(mg) = &self.message_sender {
            let _ = mg.send(BackgroundWorkerMessage::RegisterDocument(
                self.id,
                Box::new(f),
            ));
        }
    }

    /// Return style information for the given line, None if the line is not yet highlighted, out of range, or the document is plain text
    pub fn get_style_line_info(&self, line_idx: usize) -> Option<StyledLine> {
        self.line_style_cache.get(line_idx)
    }

    fn update_highlight_from(&self, line_idx: usize) {
        let (sender, receiver) = mpsc::channel();
        if let Some(tx) = self.message_sender.as_ref() {
            let _ = tx.send(BackgroundWorkerMessage::UpdateBuffer(
                self.id,
                self.file_info.syntax.clone(),
                self.rope.clone(),
                line_idx,
                self.line_style_cache.clone(),
                sender,
                self.file_info.indentation.size(),
            ));
            // block until first chunk is highlighted
            let _ = receiver.recv();
            // TODO: log error
        }
    }

    /// Update the theme of the document
    pub fn update_theme(&self, theme: &str) {
        if let Some(tx) = self.message_sender.as_ref() {
            let _ = tx.send(BackgroundWorkerMessage::UpdateTheme(theme.to_string()));
        }
        self.update_highlight_from(0);
    }

    /// Create a new Document from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(&path)?;

        let mut detector = EncodingDetector::new();
        let mut vec = Vec::new();
        file.read_to_end(&mut vec)?;

        detector.feed(&vec, true);
        let encoding = Encoding::for_bom(&vec);

        let syntax = if let Ok(s) = SYNTAXSET.find_syntax_for_file(&path) {
            s.unwrap_or_else(|| SYNTAXSET.find_syntax_plain_text())
        } else {
            SYNTAXSET.find_syntax_plain_text()
        };

        let message_sender = if let Ok(mg) = MESSAGE_SENDER.lock() {
            mg.clone()
        } else {
            None
        };

        let doc = match encoding {
            None => {
                let encoding = detector.guess(None, true);

                let rope = Rope::from_str(&encoding.decode_with_bom_removal(&vec).0);
                let linefeed = detect_linefeed(&rope.slice(..));
                let indentation = detect_indentation(&rope.slice(..));

                Self {
                    rope: rope.clone(),
                    file_info: FileInfo {
                        encoding,
                        bom: None,
                        linefeed,
                        indentation,
                        syntax,
                    },
                    selections: vec![Selection::default()],
                    file_name: Some(path.as_ref().to_path_buf()),
                    history: Default::default(),
                    id: Document::new_id(),
                    message_sender,
                    line_style_cache: StyledLinesCache::new(),
                    batch_edit: None,
                }
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

                Self {
                    rope: rope.clone(),
                    file_info: FileInfo {
                        encoding,
                        bom: Some(bom),
                        linefeed,
                        indentation,
                        syntax,
                    },
                    selections: vec![Selection::default()],
                    file_name: Some(path.as_ref().to_path_buf()),
                    history: Default::default(),
                    id: Document::new_id(),
                    message_sender,
                    line_style_cache: StyledLinesCache::new(),
                    batch_edit: None,
                }
            }
        };
        doc.update_highlight_from(0);
        Ok(doc)
    }

    fn reset_edit_stack(&mut self) {
        self.history = Default::default();
    }

    /// Save the document to the given path
    pub fn save_as(&mut self, path: &Path) -> Result<()> {
        let mut file = fs::File::create(path)?;
        let input = self.rope.to_string();
        let encoded_output = match self.file_info.encoding.name() {
            "UTF-16LE" => {
                let mut v = Vec::new();
                input
                    .encode_utf16()
                    .for_each(|i| v.extend_from_slice(&i.to_le_bytes()));
                Cow::from(v)
            }
            "UTF-16BE" => {
                let mut v = Vec::new();
                input
                    .encode_utf16()
                    .for_each(|i| v.extend_from_slice(&i.to_be_bytes()));
                Cow::from(v)
            }
            _ => self.file_info.encoding.encode(&input).0,
        };

        if let Some(bom) = &self.file_info.bom {
            file.write_all(bom)?;
        }
        file.write_all(&encoded_output)?;

        self.reset_edit_stack();
        self.file_name = Some(path.to_owned());
        Ok(())
    }

    /// Find the input string inside the Document starting at the given position and return the start and end position of the first match
    /// If cycling is true, the search will continue from the beginning of the document if the end is reached
    /// If no match is found, return None
    pub fn find_from(
        &self,
        input: &str,
        position: Position,
        cycling: bool,
    ) -> Option<(Position, Position)> {
        if input.is_empty() {
            return None;
        }

        let char_idx = self.position_to_char(position);

        for i in char_idx..self.rope.len_chars() {
            let doc = self.rope.slice(i..).chars();
            let search = input.chars();
            if search.zip(doc).all(|(ic, c)| ic == c) {
                return Some((
                    self.char_to_position(i),
                    self.char_to_position(i + input.len()),
                ));
            }
        }
        if cycling {
            for i in 0..char_idx {
                let doc = self.rope.slice(i..).chars();
                let search = input.chars();

                if search.zip(doc).all(|(ic, c)| ic == c) {
                    return Some((
                        self.char_to_position(i),
                        self.char_to_position(i + input.len()),
                    ));
                }
            }
        }

        None
    }

    /// Start a batch of edit Action
    /// The batch will be committed when end_batch_edit is called
    /// Batching permit undo and redo to treat the whole batch as a single action
    fn begin_batch_edit(&mut self, action: Action) {
        let rope = self.rope.clone();
        let selections = self.selections.clone();
        self.batch_edit = Some(BatchEdit {
            rope,
            selections: selections.clone(),
            action,
            from_char_idx: self.position_to_char(
                self.selections
                    .iter()
                    .min()
                    .map(|s| s.start())
                    .unwrap_or_default(),
            ),
            have_change: false,
        });
    }

    /// End the current batch
    fn end_batch_edit(&mut self) {
        if let Some(batch) = self.batch_edit.take() {
            if batch.have_change {
                self.history
                    .push(batch.rope, batch.selections, &batch.action);
                self.update_highlight_from(
                    self.rope
                        .char_to_line(batch.from_char_idx.min(self.rope.len_chars())),
                );
                self.batch_edit = None;
            }
        }
    }

    fn insert_at_position(&mut self, input: &str, start: Position, end: Position) {
        let start = self.position_to_char(start);
        let end = self.position_to_char(end);
        self.insert_at(input, start, end);
    }

    fn insert_at_selection(&mut self, input: &str, selection: Selection) {
        self.insert_at_position(&input, selection.start(), selection.end());
    }

    /// Modification of the document content are not saved to disk
    pub fn is_dirty(&self) -> bool {
        !self.history.is_empty()
    }

    fn insert_at(&mut self, input: &str, start: usize, end: usize) {
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
            (0..self.selections.len()).for_each(|i| {
                if sel_idx[i].0 >= end {
                    self.selections[i].head =
                        char_to_position(&self.rope.slice(..), sel_idx[i].0 - to_sub);
                }
                if sel_idx[i].1 >= end {
                    self.selections[i].tail =
                        char_to_position(&self.rope.slice(..), sel_idx[i].1 - to_sub);
                }
            });
            changed = true;
        }

        if !input.is_empty() {
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
            (0..self.selections.len()).for_each(|i| {
                if sel_idx[i].0 >= start {
                    self.selections[i].head =
                        char_to_position(&self.rope.slice(..), sel_idx[i].0 + to_add);
                }
                if sel_idx[i].1 >= start {
                    self.selections[i].tail =
                        char_to_position(&self.rope.slice(..), sel_idx[i].1 + to_add);
                }
            });
            changed = true;
        }

        if let Some(batch) = &mut self.batch_edit {
            batch.have_change = changed;
        }

        // TODO: cause performance issue
        // if changed {
        //     if let Some(batch) = &mut self.batch_edit {
        //         batch.have_change = true
        //     } else {
        //         self.history
        //             .push(histo_rope, histo_selections, &saved_action);
        //         self.update_highlight_from(self.rope.char_to_line(start));
        //     }
        // }
    }

    /// Undo the last action
    pub fn undo(&mut self) {
        if let Some((rope, selections)) = self
            .history
            .undo(self.rope.clone(), self.selections.clone())
        {
            self.rope = rope;
            self.selections = selections;
            // TODO: potential perf issue
            self.update_highlight_from(0);
        }
    }

    /// Redo the last undone action
    pub fn redo(&mut self) {
        if let Some((rope, selections)) = self.history.redo() {
            self.rope = rope;
            self.selections = selections;
            // TODO: potential perf issue
            self.update_highlight_from(0);
        }
    }

    /// Convert a [position](Position) to a char index
    pub fn position_to_char(&self, position: Position) -> usize {
        position_to_char(&self.rope.slice(..), position)
    }

    /// Convert a char index to a [position](Position)
    pub fn char_to_position(&self, char_idx: usize) -> Position {
        char_to_position(&self.rope.slice(..), char_idx)
    }

    /// Return the content of all the selections
    /// The different selections are separated by a new line
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

    /// Insert the given string at the current [selection](Selection).
    /// If the number of lines in the input string is equal to the number of selections, each line is inserted at the corresponding selection.
    /// This is mostly useful for copy/pasting in multi-cursor situation.
    pub fn insert_many(&mut self, input: &str) {
        self.begin_batch_edit(Action::Text(input.to_string()));
        if self.selections.len() > 1 && input.lines().count() == self.selections.len() {
            for (i, l) in input.lines().enumerate() {
                self.insert_at_selection(l, self.selections[i]);
            }
        } else {
            self.insert(input);
        }
        self.end_batch_edit();
    }

    /// Insert the given string at the current [selection](Selection).
    pub fn insert(&mut self, input: &str) {
        self.begin_batch_edit(Action::Text(input.to_string()));
        for i in 0..self.selections.len() {
            let selection = self.selections[i];
            let char_idx_start = self.position_to_char(selection.start());
            let char_idx_end = self.position_to_char(selection.start());
            let (input, selection) = match (
                input,
                selection.is_empty(),
                self.rope.chars_at(char_idx_start).prev(),
                self.rope.chars_at(char_idx_end).next(),
            ) {
                // insert { and do nothing because } is allready there
                ("{", true, _, Some('}')) => (input.to_string(), None),
                // } is allready there, insert nothing, just move the cursor to the right
                ("}", true, _, Some('}')) => {
                    let mut selection = selection;
                    selection.head = Position::new(selection.head.line, selection.head.column + 1);
                    selection.tail = selection.head;
                    ("".to_string(), Some(selection))
                }
                // insert {} and move the cursor to the right
                ("{", true, _, _) => {
                    let mut selection = selection;
                    selection.head = Position::new(selection.head.line, selection.head.column + 1);
                    selection.tail = selection.head;
                    ("{}".to_string(), Some(selection))
                }
                // insert new line between { and } insert two line with the correct indentation and place the cursor in between
                ("\r" | "\n" | "\r\n", _, Some('{'), Some('}')) => {
                    let indent1 = self.compute_indentation(selection.head, 1);
                    let indent2 = self.compute_indentation(selection.head, 0);
                    let first_part = format!("{}{}", self.file_info.linefeed, indent1);

                    let mut selection = selection;
                    selection.head =
                        Position::new(selection.head.line + 1, indent1.chars().count());
                    selection.tail = selection.head;
                    let second_part = format!("{}{}", self.file_info.linefeed, indent2);
                    let input = format!("{}{}", first_part, second_part);
                    (input, Some(selection))
                }
                // insert new line after a { should increment the indentation
                ("\r" | "\n" | "\r\n", _, Some('{'), _) => (
                    format!(
                        "{}{}",
                        self.file_info.linefeed,
                        self.compute_indentation(selection.head, 1),
                    ),
                    None,
                ),
                // insert new line before a } should decrement the indentation
                ("\r" | "\n" | "\r\n", _, _, Some('}')) => (
                    format!(
                        "{}{}",
                        self.file_info.linefeed,
                        self.compute_indentation(selection.head, -1),
                    ),
                    None,
                ),
                // Insert new line should preserve the indentation
                ("\r" | "\n" | "\r\n", _, _, _) => (
                    format!(
                        "{}{}",
                        self.file_info.linefeed,
                        self.compute_indentation(selection.head, 0),
                    ),
                    None,
                ),
                // TODO: find a way to not reallocate the string
                _ => (input.to_string(), None),
            };

            self.insert_at_selection(&input, self.selections[i]);
            if let Some(selection) = selection {
                self.selections[i] = selection;
            }
        }
        self.merge_selections();
        self.end_batch_edit();
    }

    /// Delete the character juste before [selections](Self::selections)
    pub fn backspace(&mut self) {
        self.begin_batch_edit(Action::Backspace);
        for i in 0..self.selections.len() {
            if self.selections[i].head == self.selections[i].tail {
                let start = self.selections[i].start();
                self.insert_at_position("", self.prev_position(start), start);
            } else {
                self.insert_at_selection("", self.selections[i]);
            }
        }
        self.merge_selections();
        self.end_batch_edit();
    }

    /// Delete the character under [selections](Self::selections)
    pub fn delete(&mut self) {
        self.begin_batch_edit(Action::Delete);
        for i in 0..self.selections.len() {
            if self.selections[i].head == self.selections[i].tail {
                let start = self.selections[i].start();
                self.insert_at_position("", start, self.next_position(start));
            } else {
                self.insert_at_selection("", self.selections[i]);
            }
        }
        self.merge_selections();
        self.end_batch_edit();
    }

    fn compute_indentation(&self, position: Position, delta: isize) -> String {
        let line = position.line;
        let col = position.column;
        let indent_len = self.line_indent_len(line).min(col);
        let indent = self.rope.line(line).slice(..indent_len).to_string();

        match delta.cmp(&0) {
            std::cmp::Ordering::Less => match self.file_info.indentation {
                Indentation::Tab(_) => {
                    let l = indent_len.saturating_sub(-delta as usize);
                    "\t".repeat(l)
                }
                Indentation::Space(x) => {
                    let l = indent_len.saturating_sub(-delta as usize * x);
                    " ".repeat(l)
                }
            },
            std::cmp::Ordering::Equal => indent,
            std::cmp::Ordering::Greater => match self.file_info.indentation {
                Indentation::Tab(_) => format!("{}{}", indent, "\t".repeat(delta as usize)),
                Indentation::Space(x) => format!("{}{}", indent, " ".repeat(delta as usize * x)),
            },
        }
    }

    /// Increase the indentation of the selected lines
    /// Insert a tab character at selections if only one line is selected or 'always' is false
    pub fn indent(&mut self, always: bool) {
        self.begin_batch_edit(Action::Tab);
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
        self.end_batch_edit();
    }

    /// decrease the indentation of the selected lines
    pub fn deindent(&mut self) {
        self.begin_batch_edit(Action::Text(String::new()));
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
        self.end_batch_edit();
    }

    /// Move all the [selections](Self::selections) in the given direction.
    /// If expand is true, only the head of the selection is moved (the cursor part)
    pub fn move_selections(&mut self, dir: MoveDirection, expand: bool) {
        self.selections = self
            .selections
            .iter()
            .map(|s| {
                let vcol = s.head.vcol;
                let mut head = match dir {
                    MoveDirection::Up => {
                        let line = s.head.line.saturating_sub(1);
                        Position::new(
                            line,
                            s.head
                                .vcol
                                .min(line_len_grapheme(&self.rope.slice(..), line)),
                        )
                    }
                    MoveDirection::Down => {
                        let line = usize::min(s.head.line + 1, self.rope.len_lines() - 1);
                        Position::new(
                            line,
                            s.head
                                .vcol
                                .min(line_len_grapheme(&self.rope.slice(..), line)),
                        )
                    }
                    MoveDirection::Left => self.prev_position(s.head),
                    MoveDirection::Right => self.next_position(s.head),
                };
                if matches!(dir, MoveDirection::Down | MoveDirection::Up) {
                    head.vcol = vcol;
                }
                let tail = if !expand { head } else { s.tail };

                Selection::new(head, tail, s.is_clone, s.generation)
            })
            .collect();

        self.merge_selections();
    }

    /// Move all the [selections](Self::selections) to the next/prev word.
    /// If expand is true, only the head of the selection is moved (the cursor part)
    pub fn move_selections_word(&mut self, dir: MoveDirection, expand: bool) {
        self.selections = self
            .selections
            .iter()
            .map(|s| {
                let head = match dir {
                    MoveDirection::Left => self.prev_word_boundary(s.head),
                    MoveDirection::Right => self.next_word_boundary(s.head),
                    _ => s.head,
                };

                let tail = if !expand { head } else { s.tail };
                Selection::new(head, tail, s.is_clone, s.generation)
            })
            .collect();

        self.merge_selections();
    }

    /// Return the start [position](Position) of the next word
    pub fn next_word_boundary(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(
            slice,
            next_word_boundary(slice, position_to_char(slice, position)),
        )
    }

    /// Return the end [position](Position) of the previous word
    pub fn prev_word_boundary(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(
            slice,
            prev_word_boundary(slice, position_to_char(slice, position)),
        )
    }

    /// Return the [position](Position) of the prev grapheme boundary
    pub fn prev_position(&self, position: Position) -> Position {
        let char_idx = self.position_to_char(position);
        self.char_to_position(prev_grapheme_boundary(&self.rope.slice(..), char_idx))
    }

    /// Return the [position](Position) of the next grapheme boundary
    pub fn next_position(&self, position: Position) -> Position {
        let char_idx = self.position_to_char(position);
        self.char_to_position(next_grapheme_boundary(&self.rope.slice(..), char_idx))
    }

    /// Return the start [position](Position) of the word pointed by the given [position](Position)
    pub fn word_start(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(slice, word_start(slice, position_to_char(slice, position)))
    }

    /// Return the end [position](Position) of the word pointed by the given [position](Position)
    pub fn word_end(&self, position: Position) -> Position {
        let slice = &self.rope.slice(..);
        char_to_position(slice, word_end(slice, position_to_char(slice, position)))
    }

    /// Select the word pointed by the given [position](Position)
    pub fn select_word(&mut self, position: Position) {
        let tail = self.word_start(position);
        let head = self.word_end(position);
        self.selections = vec![Selection {
            head,
            tail,
            is_clone: false,
            generation: 0,
        }]
    }

    /// Expand the selections to the word bondary pointed by the given [position](Position)
    pub fn expand_selection_by_word(&mut self, position: Position) {
        match position {
            p if p < self.selections[0].tail => {
                let end = self.selections[0].end();
                self.selections[0].head = self.word_start(p);
                self.selections[0].tail = end;
            }
            p if p > self.selections[0].tail => {
                let start = self.selections[0].start();
                self.selections[0].head = self.word_end(p);
                self.selections[0].tail = start;
            }
            _ => (),
        }
    }

    /// Expand the selections to the line pointed by the given [position](Position)
    pub fn expand_selection_by_line(&mut self, position: Position) {
        match position {
            p if p < self.selections[0].tail => {
                let end = self.selections[0].end();
                self.selections[0].head = self.line_start(p.line);
                self.selections[0].tail = end;
            }
            p if p > self.selections[0].tail => {
                let start = self.selections[0].start();
                self.selections[0].head = self.line_end_full(p.line);
                self.selections[0].tail = start;
            }
            _ => (),
        }
    }

    /// return the start [position](Position) of the line pointed by the given [position](Position)
    /// (stupid? it could be just Position.line)
    pub fn line_start(&mut self, line: usize) -> Position {
        char_to_position(&self.rope.slice(..), self.rope.line_to_char(line))
    }

    /// return the line indentation length in char
    pub fn line_indent_len(&self, line: usize) -> usize {
        get_line_start_boundary(&self.rope.slice(..), line)
    }

    /// return the end position of the line
    pub fn line_end(&mut self, line: usize) -> Position {
        char_to_position(
            &self.rope.slice(..),
            self.rope.line_to_char(line) + line_len_char(&self.rope.slice(..), line),
        )
    }

    /// return the end position of the line including end of line characters
    pub fn line_end_full(&mut self, line: usize) -> Position {
        self.line_start(line + 1)
    }

    pub fn select_line(&mut self, line: usize) {
        let tail = self.line_start(line);
        let head = self.line_end_full(line);
        self.selections = vec![Selection {
            head,
            tail,
            is_clone: false,
            generation: 0,
        }]
    }

    pub fn select_all(&mut self) {
        let tail = char_to_position(&self.rope.slice(..), 0);
        let head = char_to_position(&self.rope.slice(..), self.rope.len_chars());
        self.selections = vec![Selection {
            head,
            tail,
            is_clone: false,
            generation: 0,
        }]
    }

    /// Specify the main selection. Also cancel multi-cursor
    pub fn set_main_selection(&mut self, head: Position, tail: Position) {
        self.selections = vec![Selection {
            head,
            tail,
            is_clone: false,
            generation: 0,
        }]
    }

    pub fn cancel_multi_cursor(&mut self) {
        self.selections = self
            .selections
            .iter()
            .filter(|s| !s.is_clone)
            .copied()
            .collect();
    }

    /// duplicate the selection in the given direction
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
                news.is_clone = true;
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
                news.is_clone = true;
                if news.head.line < s.head.line {
                    self.selections.push(news);
                }
            }
            _ => (),
        }

        self.merge_selections();
    }

    /// ducplicate the selection based on the currently selected text
    pub fn duplicate_selection_for_selected_text(&mut self) {
        let start = self.position_to_char(self.selections[0].start());
        let end = self.position_to_char(self.selections[0].end());
        let content = self.rope.slice(start..end).to_string();
        let mut s = self
            .selections
            .iter()
            .max_by_key(|s| s.generation)
            .unwrap()
            .duplicate();
        let next = self.find_from(&content, s.end(), true);
        if let Some((start, end)) = next {
            s.tail = start;
            s.head = end;
            self.selections.push(s);
            self.merge_selections();
        }
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

    fn merge_selections(&mut self) {
        if self.selections.len() == 1 {
            return;
        }
        self.selections.sort_unstable_by_key(|a| a.start());
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

    fn line_has_tab(&self, line_idx: usize) -> bool {
        for c in self.rope.line(line_idx).chunks() {
            if c.contains('\t') {
                return true;
            }
        }
        false
    }

    /// return the content of the line taking into account the tab character.
    /// The Tab character is replaced by the number of space corresponding to the tab size, according to the elastic tab stop behavior
    pub fn get_visible_line(&self, line_idx: usize) -> Cow<'_, str> {
        if self.line_has_tab(line_idx) {
            let indent_len = self.file_info.indentation.size();
            let mut s = String::with_capacity(self.rope.line(line_idx).len_chars());
            let mut offset = 0;
            for c in self.rope.line(line_idx).chars() {
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
            match self.rope.line(line_idx).as_str() {
                Some(s) => s.into(),
                None => self.rope.line(line_idx).to_string().into(),
            }
        }
    }

    /// convert the column index to a column index into a visible line
    pub fn col_to_vcol(&self, line_idx: usize, col_idx: usize) -> usize {
        let slice = self.rope.line(line_idx);
        let tabl_len = self.file_info.indentation.size();

        let mut vcol = 0;
        for (i, j) in rope_utils::NextGraphemeIdxIterator::new(&slice)
            .take(col_idx + 1)
            .tuple_windows()
        {
            match slice.slice(i..j).char(0) {
                '\t' => vcol += tabl_len - (vcol % tabl_len),
                _ => vcol += 1,
            }
        }
        vcol
    }

    /// Convert the visible column index to a real column index
    pub fn vcol_to_col(&self, line_idx: usize, vcol_idx: usize) -> usize {
        let slice = self.rope.line(line_idx);
        let tabl_len = self.file_info.indentation.size();
        let mut vcol = 0;
        let mut col = 0;
        for (i, j) in rope_utils::NextGraphemeIdxIterator::new(&slice).tuple_windows() {
            match slice.slice(i..j).char(0) {
                '\t' => vcol += tabl_len - (vcol % tabl_len),
                _ => vcol += 1,
            }
            if vcol > vcol_idx {
                return col;
            }
            col += 1;
        }
        col
    }

    /// return the byte index of the visible column index
    pub fn vcol_to_byte(&self, line_idx: usize, vcol_idx: usize) -> usize {
        let rope = Rope::from_str(&self.get_visible_line(line_idx));
        let char_idx = rope_utils::NextGraphemeIdxIterator::new(&rope.slice(..))
            .nth(vcol_idx)
            .unwrap();
        rope.char_to_byte(char_idx)
    }

    /// return the visible column index of the byte index
    pub fn byte_to_vcol(&self, line_idx: usize, byte_idx: usize) -> usize {
        let rope = Rope::from_str(&self.get_visible_line(line_idx));
        let char_idx = rope.byte_to_char(byte_idx);
        rope_utils::NextGraphemeIdxIterator::new(&rope.slice(..))
            .take_while(|i| *i < char_idx)
            .count()
    }

    pub fn col_to_byte(&self, line_idx: usize, col_idx: usize) -> usize {
        let vcol_idx = self.col_to_vcol(line_idx, col_idx);
        self.vcol_to_byte(line_idx, vcol_idx)
    }

    pub fn byte_to_col(&self, line_idx: usize, byte_idx: usize) -> usize {
        let vcol_idx = self.byte_to_vcol(line_idx, byte_idx);
        self.vcol_to_col(line_idx, vcol_idx)
    }

    /// return the visible column position corresponding of the byte_idx of the line
    /// this function take into account the tab character and elastic tab stop behavior
    pub fn byte_to_visible_col(&self, line_idx: usize, byte_idx: usize) -> usize {
        byte_to_visible_col(
            self.rope.line(line_idx).bytes(),
            byte_idx,
            self.file_info.indentation.size(),
        )
    }

    /// return the byte index position of the corresponding visible column.
    /// this function take into account the tab character and elastic tab stop behavior
    pub fn visible_col_to_byte(&self, line_idx: usize, vcol_idx: usize) -> usize {
        let slice = self.rope.line(line_idx);
        let mut byte_idx = 0;
        let mut vcol = 0;
        let tabl_len = self.file_info.indentation.size();
        while vcol < vcol_idx {
            let i = rope_utils::next_grapheme_boundary_byte(&slice, byte_idx);

            if i == byte_idx {
                // we have reached eol
                return i;
            }

            match slice.byte_slice(byte_idx..i).char(0) {
                '\t' => vcol += tabl_len - (vcol % tabl_len),
                _ => vcol += 1,
            }
            byte_idx = i;
        }
        byte_idx
    }
}

fn byte_to_visible_col(input: impl Iterator<Item = u8>, byte_idx: usize, tab_len: usize) -> usize {
    let mut col = 0;
    for c in input.take(byte_idx) {
        if c == b'\t' {
            col += tab_len - (col % tab_len);
        } else {
            col += 1;
        }
    }
    col
}

#[test]
fn test_byte_to_visible_col() {
    let s = "a\tb\tc";
    assert_eq!(byte_to_visible_col(s.bytes(), 0, 4), 0);
    assert_eq!(byte_to_visible_col(s.bytes(), 1, 4), 1);
    assert_eq!(byte_to_visible_col(s.bytes(), 2, 4), 4);
    assert_eq!(byte_to_visible_col(s.bytes(), 3, 4), 5);
    assert_eq!(byte_to_visible_col(s.bytes(), 4, 4), 8);
    assert_eq!(byte_to_visible_col(s.bytes(), 5, 4), 9);
}

#[test]
fn test_visible_col_to_byte() {
    let mut s = Document::new(Indentation::Tab(4));
    s.insert("a\tb\tc");
    // "a   b   c"
    assert_eq!(s.visible_col_to_byte(0, 0), 0);
    assert_eq!(s.visible_col_to_byte(0, 1), 1);
    assert_eq!(s.visible_col_to_byte(0, 2), 2);
    assert_eq!(s.visible_col_to_byte(0, 3), 2);
    assert_eq!(s.visible_col_to_byte(0, 4), 2);
    assert_eq!(s.visible_col_to_byte(0, 5), 3);
    assert_eq!(s.visible_col_to_byte(0, 6), 4);
    assert_eq!(s.visible_col_to_byte(0, 7), 4);
    assert_eq!(s.visible_col_to_byte(0, 8), 4);
    assert_eq!(s.visible_col_to_byte(0, 9), 5);
}

pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default, Debug, Clone, Copy, Eq)]
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
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.column.cmp(&other.column)
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

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub head: Position,
    pub tail: Position,
    is_clone: bool,
    generation: usize,
}

impl PartialOrd for Selection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Selection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tail.cmp(&other.head)
    }
}

impl From<(Position, Position)> for Selection {
    fn from((head, tail): (Position, Position)) -> Self {
        Self {
            head,
            tail,
            is_clone: false,
            generation: 0,
        }
    }
}

impl Selection {
    fn new(head: Position, tail: Position, is_clone: bool, generation: usize) -> Self {
        Self {
            head,
            tail,
            is_clone,
            generation,
        }
    }
    pub fn duplicate(&self) -> Self {
        Self {
            head: self.head,
            tail: self.tail,
            is_clone: true,
            generation: self.generation + 1,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
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
                if self.is_clone != other.is_clone {
                    self.is_clone = false;
                    self.generation = 0;
                }
                return true;
            }
        } else if other.tail < self.head {
            self.head = other.head.max(self.head);
            if self.is_clone != other.is_clone {
                self.is_clone = false;
                self.generation = 0;
            }
            return true;
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
fn line_len_char(rope: &RopeSlice, line_idx: usize) -> usize {
    let mut r = rope.line(line_idx).chars().collect::<Vec<char>>();
    r.reverse();
    let linefeed_len = match (r.get(1), r.first()) {
        (Some('\u{000D}'), Some('\u{000A}')) => 2,
        (_, Some('\u{000A}')) => 1,
        (_, Some('\u{000D}')) => 1,
        (_, _) => 0,
    };
    r.len() - linefeed_len
}

fn line_len_char_full(rope: &RopeSlice, line_idx: usize) -> usize {
    rope.line_to_char(line_idx + 1) - rope.line_to_char(line_idx)
}

fn position_to_char(slice: &RopeSlice, position: Position) -> usize {
    let l = slice.line_to_char(position.line);
    l + grapheme_to_char(&slice.line(position.line), position.column)
}

#[test]
fn test_position_to_char() {
    let r = Rope::from("aaaa\r\n\r\n\r\naaaa\r\n");
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
            (
                l.next().unwrap().parse::<usize>().unwrap(),
                l.next().unwrap().parse::<usize>().unwrap(),
                l.next().unwrap().parse::<usize>().unwrap(),
            )
        })
        .collect::<Vec<(usize, usize, usize)>>();
    let rope = Rope::from(source);
    let slice = rope.slice(..);

    for e in expected {
        assert_eq!(
            char_to_position(&slice, rope.byte_to_char(e.0)),
            Position::new(e.1, e.2),
            "testing byte index {} (char {}) for line {}",
            e.0,
            rope.byte_to_char(e.0),
            slice.line(e.1)
        );
    }
}

fn char_to_position(rope: &RopeSlice, char_idx: usize) -> Position {
    let line = rope.char_to_line(char_idx.min(rope.len_chars()));
    //let column = print_positions::print_positions(&rope.line(line).chars().take(char_idx).collect::<String>()).count();
    let column = char_to_grapheme(&rope.line(line), char_idx - rope.line_to_char(line));
    Position::new(line, column)
}

fn line_len_grapheme(rope: &RopeSlice, line_idx: usize) -> usize {
    //line_len_char(rope, line_idx)
    char_to_grapheme(&rope.line(line_idx), line_len_char(rope, line_idx))
}

fn line_len_grapheme_full(rope: &RopeSlice, line_idx: usize) -> usize {
    //line_len_char(rope, line_idx)
    char_to_grapheme(&rope.line(line_idx), line_len_char_full(rope, line_idx))
}

// #[test]
// fn test_position_from_char_idx_point() {
//     let rope = Rope::from("  Misc Sm Circles:  ｡ ⋄ ° ﾟ ˚ ﹾ");
//     assert_eq!(char_to_position(&rope.slice(..), 27), Position::new(0, 27));
// }
#[cfg(test)]
mod test {
    use ropey::Rope;

    use crate::{rope_utils::char_to_grapheme, Document};

    #[test]
    fn test_char_to_grapheme() {
        let rope = Rope::from("  Diamonds:         ⋄ ᛜ ⌔ ◇ ⟐ ◈ ◆   ◊");
        let column = char_to_grapheme(&rope.slice(..), 27);
        assert_eq!(column, 27);
    }

    #[test]
    fn find_from() {
        let mut doc = Document::default();
        doc.insert("hello world \n hell \n hello");
        let s = "hello";
        let idx = doc.find_from(s, doc.char_to_position(0), false).unwrap();
        assert_eq!(idx, (doc.char_to_position(0), doc.char_to_position(5)));
        let idx = doc.find_from(s, idx.1, false).unwrap();
        assert_eq!(idx, (doc.char_to_position(21), doc.char_to_position(26)));
    }

    #[test]
    fn find_from_no_match() {
        let mut doc = Document::default();
        doc.insert("hello world \n hell \n hello");
        let s = "wrold";
        let idx = doc.find_from(s, doc.char_to_position(0), false);
        assert_eq!(idx, None);
    }
}
