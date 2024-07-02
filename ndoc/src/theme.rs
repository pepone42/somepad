use std::str::FromStr;

use once_cell::sync::Lazy;

use nonepad_vscodetheme::*;
use syntect::highlighting::{
    ScopeSelector, ScopeSelectors, StyleModifier, ThemeItem, ThemeSettings,
};

#[derive(Debug)]
pub struct Theme {
    pub vscode: VSCodeTheme,
    pub style: syntect::highlighting::Theme,
}

fn color_from_str(col: &str) -> Option<syntect::highlighting::Color> {
    if let Some('#') = col.chars().next() {
        if col.len() == 7 {
            let r = u8::from_str_radix(&col[1..=2], 16).ok()?;
            let g = u8::from_str_radix(&col[3..=4], 16).ok()?;
            let b = u8::from_str_radix(&col[5..=6], 16).ok()?;
            let a = 255;
            return Some(syntect::highlighting::Color { r, g, b, a });
        } else if col.len() == 9 {
            let r = u8::from_str_radix(&col[1..=2], 16).ok()?;
            let g = u8::from_str_radix(&col[3..=4], 16).ok()?;
            let b = u8::from_str_radix(&col[5..=6], 16).ok()?;
            let a = u8::from_str_radix(&col[7..=8], 16).ok()?;
            return Some(syntect::highlighting::Color { r, g, b, a });
        }
    }
    None
}

impl Default for Theme {
    fn default() -> Self {
        let vscode: VSCodeTheme = VSCodeTheme::default();
        let mut style = syntect::highlighting::Theme {
            name: None,
            author: None,
            settings: ThemeSettings::default(),
            scopes: Vec::new(),
        };
        for token in &vscode.token_colors {
            let scope = ScopeSelectors {
                selectors: token
                    .scope
                    .iter()
                    .map(|s| ScopeSelector::from_str(s).unwrap())
                    .collect::<Vec<ScopeSelector>>(),
            };
            //let scope = ScopeSelectors::from_str(&token.scope.join(" ")).unwrap();
            let foreground = token.settings.foreground.clone().map(|c| {
                color_from_str(&c).unwrap()
            });
            //.unwrap_or_else(|| druid::piet::Color::from_hex_str(&t.colors.editor_foreground).unwrap())
            //let bgcol = druid::piet::Color::from_hex_str(&t.colors.editor_background).unwrap().as_rgba8();
            let mut font_style = syntect::highlighting::FontStyle::empty(); //= syntect::highlighting::FontStyle::Regular;

            if let Some(fs) = token.settings.font_style.clone() {
                if fs.contains("italic") {
                    font_style.toggle(syntect::highlighting::FontStyle::ITALIC);
                }
                if fs.contains("bold") {
                    font_style.toggle(syntect::highlighting::FontStyle::BOLD);
                }
                if fs.contains("underline") {
                    font_style.toggle(syntect::highlighting::FontStyle::UNDERLINE);
                }
            }
            let style_modifier = StyleModifier {
                foreground,
                background: None,
                font_style: Some(font_style),
            };
            let theme_item = ThemeItem {
                scope,
                style: style_modifier,
            };
            style.scopes.push(theme_item)
        }
        style.settings.foreground = {
            color_from_str(&vscode.colors.foreground)
        };

        Self { vscode, style }
    }
}

pub static THEME: Lazy<Theme> = Lazy::new(Theme::default);

// pub const FOCUS_BORDER: Key<Color> = Key::new("focusBorder");
// pub const FOREGROUND: Key<Color> = Key::new("foreground");
// pub const SELECTION_BACKGROUND: Key<Color> = Key::new("selection.background");
// pub const WIDGET_SHADOW: Key<Color> = Key::new("widget.shadow");
// pub const TEXT_LINK_ACTIVE_FOREGROUND: Key<Color> = Key::new("textLink.activeForeground");
// pub const TEXT_LINK_FOREGROUND: Key<Color> = Key::new("textLink.foreground");
// pub const TEXT_PREFORMAT_FOREGROUND: Key<Color> = Key::new("textPreformat.foreground");
// pub const BUTTON_BACKGROUND: Key<Color> = Key::new("button.background");
// pub const BUTTON_FOREGROUND: Key<Color> = Key::new("button.foreground");
// pub const BUTTON_HOVER_BACKGROUND: Key<Color> = Key::new("button.hoverBackground");
// pub const DROPDOWN_BACKGROUND: Key<Color> = Key::new("dropdown.background");
// pub const DROPDOWN_LIST_BACKGROUND: Key<Color> = Key::new("dropdown.listBackground");
// pub const INPUT_BACKGROUND: Key<Color> = Key::new("input.background");
// pub const INPUT_BORDER: Key<Color> = Key::new("input.border");
// pub const INPUT_FOREGROUND: Key<Color> = Key::new("input.foreground");
// pub const INPUT_PLACEHOLDER_FOREGROUND: Key<Color> = Key::new("input.placeholderForeground");
// pub const SCROLLBAR_SHADOW: Key<Color> = Key::new("scrollbar.shadow");
// pub const SCROLLBAR_SLIDER_ACTIVE_BACKGROUND: Key<Color> = Key::new("scrollbarSlider.activeBackground");
// pub const SCROLLBAR_SLIDER_BACKGROUND: Key<Color> = Key::new("scrollbarSlider.background");
// pub const SCROLLBAR_SLIDER_HOVER_BACKGROUND: Key<Color> = Key::new("scrollbarSlider.hoverBackground");
// pub const BADGE_FOREGROUND: Key<Color> = Key::new("badge.foreground");
// pub const BADGE_BACKGROUND: Key<Color> = Key::new("badge.background");
// pub const PROGRESS_BAR_BACKGROUND: Key<Color> = Key::new("progressBar.background");
// pub const LIST_ACTIVE_SELECTION_BACKGROUND: Key<Color> = Key::new("list.activeSelectionBackground");
// pub const LIST_ACTIVE_SELECTION_FOREGROUND: Key<Color> = Key::new("list.activeSelectionForeground");
// pub const LIST_INACTIVE_SELECTION_BACKGROUND: Key<Color> = Key::new("list.inactiveSelectionBackground");
// pub const LIST_INACTIVE_SELECTION_FOREGROUND: Key<Color> = Key::new("list.inactiveSelectionForeground");
// pub const LIST_HOVER_FOREGROUND: Key<Color> = Key::new("list.hoverForeground");
// pub const LIST_FOCUS_FOREGROUND: Key<Color> = Key::new("list.focusForeground");
// pub const LIST_FOCUS_BACKGROUND: Key<Color> = Key::new("list.focusBackground");
// pub const LIST_HOVER_BACKGROUND: Key<Color> = Key::new("list.hoverBackground");
// pub const LIST_DROP_BACKGROUND: Key<Color> = Key::new("list.dropBackground");
// pub const LIST_HIGHLIGHT_FOREGROUND: Key<Color> = Key::new("list.highlightForeground");
// pub const LIST_ERROR_FOREGROUND: Key<Color> = Key::new("list.errorForeground");
// pub const LIST_WARNING_FOREGROUND: Key<Color> = Key::new("list.warningForeground");
// pub const ACTIVITY_BAR_BACKGROUND: Key<Color> = Key::new("activityBar.background");
// pub const ACTIVITY_BAR_DROP_BACKGROUND: Key<Color> = Key::new("activityBar.dropBackground");
// pub const ACTIVITY_BAR_FOREGROUND: Key<Color> = Key::new("activityBar.foreground");
// pub const ACTIVITY_BAR_BADGE_BACKGROUND: Key<Color> = Key::new("activityBarBadge.background");
// pub const ACTIVITY_BAR_BADGE_FOREGROUND: Key<Color> = Key::new("activityBarBadge.foreground");
// pub const SIDE_BAR_BACKGROUND: Key<Color> = Key::new("sideBar.background");
// pub const SIDE_BAR_FOREGROUND: Key<Color> = Key::new("sideBar.foreground");
// pub const SIDE_BAR_SECTION_HEADER_BACKGROUND: Key<Color> = Key::new("sideBarSectionHeader.background");
// pub const SIDE_BAR_SECTION_HEADER_FOREGROUND: Key<Color> = Key::new("sideBarSectionHeader.foreground");
// pub const SIDE_BAR_TITLE_FOREGROUND: Key<Color> = Key::new("sideBarTitle.foreground");
// pub const EDITOR_GROUP_BORDER: Key<Color> = Key::new("editorGroup.border");
// pub const EDITOR_GROUP_DROP_BACKGROUND: Key<Color> = Key::new("editorGroup.dropBackground");
// pub const EDITOR_GROUP_HEADER_NO_TABS_BACKGROUND: Key<Color> = Key::new("editorGroupHeader.noTabsBackground");
// pub const EDITOR_GROUP_HEADER_TABS_BACKGROUND: Key<Color> = Key::new("editorGroupHeader.tabsBackground");
// pub const TAB_ACTIVE_BACKGROUND: Key<Color> = Key::new("tab.activeBackground");
// pub const TAB_ACTIVE_FOREGROUND: Key<Color> = Key::new("tab.activeForeground");
// pub const TAB_BORDER: Key<Color> = Key::new("tab.border");
// pub const TAB_ACTIVE_BORDER: Key<Color> = Key::new("tab.activeBorder");
// pub const TAB_UNFOCUSED_ACTIVE_BORDER: Key<Color> = Key::new("tab.unfocusedActiveBorder");
// pub const TAB_INACTIVE_BACKGROUND: Key<Color> = Key::new("tab.inactiveBackground");
// pub const TAB_INACTIVE_FOREGROUND: Key<Color> = Key::new("tab.inactiveForeground");
// pub const TAB_UNFOCUSED_ACTIVE_FOREGROUND: Key<Color> = Key::new("tab.unfocusedActiveForeground");
// pub const TAB_UNFOCUSED_INACTIVE_FOREGROUND: Key<Color> = Key::new("tab.unfocusedInactiveForeground");
// pub const EDITOR_BACKGROUND: Key<Color> = Key::new("editor.background");
// pub const EDITOR_FOREGROUND: Key<Color> = Key::new("editor.foreground");
// pub const EDITOR_HOVER_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.hoverHighlightBackground");
// pub const EDITOR_FIND_MATCH_BACKGROUND: Key<Color> = Key::new("editor.findMatchBackground");
// pub const EDITOR_FIND_MATCH_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.findMatchHighlightBackground");
// pub const EDITOR_FIND_RANGE_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.findRangeHighlightBackground");
// pub const EDITOR_LINE_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.lineHighlightBackground");
// pub const EDITOR_LINE_HIGHLIGHT_BORDER: Key<Color> = Key::new("editor.lineHighlightBorder");
// pub const EDITOR_INACTIVE_SELECTION_BACKGROUND: Key<Color> = Key::new("editor.inactiveSelectionBackground");
// pub const EDITOR_SELECTION_BACKGROUND: Key<Color> = Key::new("editor.selectionBackground");
// pub const EDITOR_SELECTION_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.selectionHighlightBackground");
// pub const EDITOR_RANGE_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.rangeHighlightBackground");
// pub const EDITOR_WORD_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("editor.wordHighlightBackground");
// pub const EDITOR_WORD_HIGHLIGHT_STRONG_BACKGROUND: Key<Color> = Key::new("editor.wordHighlightStrongBackground");
// pub const EDITOR_ERROR_FOREGROUND: Key<Color> = Key::new("editorError.foreground");
// pub const EDITOR_ERROR_BORDER: Key<Color> = Key::new("editorError.border");
// pub const EDITOR_WARNING_FOREGROUND: Key<Color> = Key::new("editorWarning.foreground");
// pub const EDITOR_INFO_FOREGROUND: Key<Color> = Key::new("editorInfo.foreground");
// pub const EDITOR_WARNING_BORDER: Key<Color> = Key::new("editorWarning.border");
// pub const EDITOR_CURSOR_FOREGROUND: Key<Color> = Key::new("editorCursor.foreground");
// pub const EDITOR_INDENT_GUIDE_BACKGROUND: Key<Color> = Key::new("editorIndentGuide.background");
// pub const EDITOR_LINE_NUMBER_FOREGROUND: Key<Color> = Key::new("editorLineNumber.foreground");
// pub const EDITOR_WHITESPACE_FOREGROUND: Key<Color> = Key::new("editorWhitespace.foreground");
// pub const EDITOR_OVERVIEW_RULER_BORDER: Key<Color> = Key::new("editorOverviewRuler.border");
// pub const EDITOR_OVERVIEW_RULER_CURRENT_CONTENT_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.currentContentForeground");
// pub const EDITOR_OVERVIEW_RULER_INCOMING_CONTENT_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.incomingContentForeground");
// pub const EDITOR_OVERVIEW_RULER_FIND_MATCH_FOREGROUND: Key<Color> = Key::new("editorOverviewRuler.findMatchForeground");
// pub const EDITOR_OVERVIEW_RULER_RANGE_HIGHLIGHT_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.rangeHighlightForeground");
// pub const EDITOR_OVERVIEW_RULER_SELECTION_HIGHLIGHT_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.selectionHighlightForeground");
// pub const EDITOR_OVERVIEW_RULER_WORD_HIGHLIGHT_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.wordHighlightForeground");
// pub const EDITOR_OVERVIEW_RULER_WORD_HIGHLIGHT_STRONG_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.wordHighlightStrongForeground");
// pub const EDITOR_OVERVIEW_RULER_MODIFIED_FOREGRUND: Key<Color> = Key::new("editorOverviewRuler.modifiedForeground");
// pub const EDITOR_OVERVIEW_RULER_ADDED_FOREGROUND: Key<Color> = Key::new("editorOverviewRuler.addedForeground");
// pub const EDITOR_OVERVIEW_RULER_DELETED_FOREGROUND: Key<Color> = Key::new("editorOverviewRuler.deletedForeground");
// pub const EDITOR_OVERVIEW_RULER_ERROR_FOREGROUND: Key<Color> = Key::new("editorOverviewRuler.errorForeground");
// pub const EDITOR_OVERVIEW_RULER_WARNING_FOREGROUND: Key<Color> = Key::new("editorOverviewRuler.warningForeground");
// pub const EDITOR_OVERVIEW_RULER_INFO_FOREGROUND: Key<Color> = Key::new("editorOverviewRuler.infoForeground");
// pub const EDITOR_OVERVIEW_RULER_BRACKET_MATCH_FOREGROUND: Key<Color> =
//     Key::new("editorOverviewRuler.bracketMatchForeground");
// pub const EDITOR_GUTTER_MODIFIED_BACKGROUND: Key<Color> = Key::new("editorGutter.modifiedBackground");
// pub const EDITOR_GUTTER_ADDED_BACKGROUND: Key<Color> = Key::new("editorGutter.addedBackground");
// pub const EDITOR_GUTTER_DELETED_BACKGROUND: Key<Color> = Key::new("editorGutter.deletedBackground");
// pub const DIFF_EDITOR_INSERTED_TEXT_BACKGROUND: Key<Color> = Key::new("diffEditor.insertedTextBackground");
// pub const DIFF_EDITOR_REMOVED_TEXT_BACKGROUND: Key<Color> = Key::new("diffEditor.removedTextBackground");
// pub const EDITOR_WIDGET_BACKGROUND: Key<Color> = Key::new("editorWidget.background");
// pub const EDITOR_WIDGET_BORDER: Key<Color> = Key::new("editorWidget.border");
// pub const EDITOR_SUGGEST_WIDGET_BACKGROUND: Key<Color> = Key::new("editorSuggestWidget.background");
// pub const PEEK_VIEW_BORDER: Key<Color> = Key::new("peekView.border");
// pub const PEEK_VIEW_EDITOR_MATCH_HIGHLIGHT_BACKGROUND: Key<Color> = Key::new("peekViewEditor.matchHighlightBackground");
// pub const PEEK_VIEW_EDITOR_GUTTER_BACKGROUND: Key<Color> = Key::new("peekViewEditorGutter.background");
// pub const PEEK_VIEW_EDITOR_BACKGROUND: Key<Color> = Key::new("peekViewEditor.background");
// pub const PEEK_VIEW_RESULT_BACKGROUND: Key<Color> = Key::new("peekViewResult.background");
// pub const PEEK_VIEW_TITLE_BACKGROUND: Key<Color> = Key::new("peekViewTitle.background");
// pub const MERGE_CURRENT_HEADER_BACKGROUND: Key<Color> = Key::new("merge.currentHeaderBackground");
// pub const MERGE_CURRENT_CONTENT_BACKGROUND: Key<Color> = Key::new("merge.currentContentBackground");
// pub const MERGE_INCOMING_HEADER_BACKGROUND: Key<Color> = Key::new("merge.incomingHeaderBackground");
// pub const MERGE_INCOMING_CONTENT_BACKGROUND: Key<Color> = Key::new("merge.incomingContentBackground");
// pub const PANEL_BACKGROUND: Key<Color> = Key::new("panel.background");
// pub const PANEL_BORDER: Key<Color> = Key::new("panel.border");
// pub const PANEL_TITLE_ACTIVE_BORDER: Key<Color> = Key::new("panelTitle.activeBorder");
// pub const STATUS_BAR_BACKGROUND: Key<Color> = Key::new("statusBar.background");
// pub const STATUS_BAR_DEBUGGING_BACKGROUND: Key<Color> = Key::new("statusBar.debuggingBackground");
// pub const STATUS_BAR_DEBUGGING_FOREGROUND: Key<Color> = Key::new("statusBar.debuggingForeground");
// pub const STATUS_BAR_NO_FOLDER_FOREGROUND: Key<Color> = Key::new("statusBar.noFolderForeground");
// pub const STATUS_BAR_NO_FOLDER_BACKGROUND: Key<Color> = Key::new("statusBar.noFolderBackground");
// pub const STATUS_BAR_FOREGROUND: Key<Color> = Key::new("statusBar.foreground");
// pub const STATUS_BAR_ITEM_ACTIVE_BACKGROUND: Key<Color> = Key::new("statusBarItem.activeBackground");
// pub const STATUS_BAR_ITEM_HOVER_BACKGROUND: Key<Color> = Key::new("statusBarItem.hoverBackground");
// pub const STATUS_BAR_ITEM_PROMINENT_BACKGROUND: Key<Color> = Key::new("statusBarItem.prominentBackground");
// pub const STATUS_BAR_ITEM_PROMINENT_HOVER_BACKGROUND: Key<Color> = Key::new("statusBarItem.prominentHoverBackground");
// pub const STATUS_BAR_BORDER: Key<Color> = Key::new("statusBar.border");
// pub const TITLE_BAR_ACTIVE_BACKGROUND: Key<Color> = Key::new("titleBar.activeBackground");
// pub const TITLE_BAR_ACTIVE_FOREGROUND: Key<Color> = Key::new("titleBar.activeForeground");
// pub const TITLE_BAR_INACTIVE_BACKGROUND: Key<Color> = Key::new("titleBar.inactiveBackground");
// pub const TITLE_BAR_INACTIVE_FOREGROUND: Key<Color> = Key::new("titleBar.inactiveForeground");
// pub const NOTIFICATION_CENTER_HEADER_FOREGROUND: Key<Color> = Key::new("notificationCenterHeader.foreground");
// pub const NOTIFICATION_CENTER_HEADER_BACKGROUND: Key<Color> = Key::new("notificationCenterHeader.background");
// pub const EXTENSION_BUTTON_PROMINENT_FOREGROUND: Key<Color> = Key::new("extensionButton.prominentForeground");
// pub const EXTENSION_BUTTON_PROMINENT_BACKGROUND: Key<Color> = Key::new("extensionButton.prominentBackground");
// pub const EXTENSION_BUTTON_PROMINENT_HOVER_BACKGROUND: Key<Color> =
//     Key::new("extensionButton.prominentHoverBackground");
// pub const PICKER_GROUP_BORDER: Key<Color> = Key::new("pickerGroup.border");
// pub const PICKER_GROUP_FOREGROUND: Key<Color> = Key::new("pickerGroup.foreground");
// pub const TERMINAL_ANSI_BRIGHT_BLACK: Key<Color> = Key::new("terminal.ansiBrightBlack");
// pub const TERMINAL_ANSI_BLACK: Key<Color> = Key::new("terminal.ansiBlack");
// pub const TERMINAL_ANSI_BLUE: Key<Color> = Key::new("terminal.ansiBlue");
// pub const TERMINAL_ANSI_BRIGHT_BLUE: Key<Color> = Key::new("terminal.ansiBrightBlue");
// pub const TERMINAL_ANSI_BRIGHT_CYAN: Key<Color> = Key::new("terminal.ansiBrightCyan");
// pub const TERMINAL_ANSI_CYAN: Key<Color> = Key::new("terminal.ansiCyan");
// pub const TERMINAL_ANSI_BRIGHT_MAGENTA: Key<Color> = Key::new("terminal.ansiBrightMagenta");
// pub const TERMINAL_ANSI_MAGENTA: Key<Color> = Key::new("terminal.ansiMagenta");
// pub const TERMINAL_ANSI_BRIGHT_RED: Key<Color> = Key::new("terminal.ansiBrightRed");
// pub const TERMINAL_ANSI_RED: Key<Color> = Key::new("terminal.ansiRed");
// pub const TERMINAL_ANSI_YELLOW: Key<Color> = Key::new("terminal.ansiYellow");
// pub const TERMINAL_ANSI_BRIGHT_YELLOW: Key<Color> = Key::new("terminal.ansiBrightYellow");
// pub const TERMINAL_ANSI_BRIGHT_GREEN: Key<Color> = Key::new("terminal.ansiBrightGreen");
// pub const TERMINAL_ANSI_GREEN: Key<Color> = Key::new("terminal.ansiGreen");
// pub const TERMINAL_ANSI_WHITE: Key<Color> = Key::new("terminal.ansiWhite");
// pub const TERMINAL_SELECTION_BACKGROUND: Key<Color> = Key::new("terminal.selectionBackground");
// pub const TERMINAL_CURSOR_BACKGROUND: Key<Color> = Key::new("terminalCursor.background");
// pub const TERMINAL_CURSOR_FOREGROUND: Key<Color> = Key::new("terminalCursor.foreground");
// pub const GIT_DECORATION_MODIFIED_RESOURCE_FOREGROUND: Key<Color> =
//     Key::new("gitDecoration.modifiedResourceForeground");
// pub const GIT_DECORATION_DELETED_RESOURCE_FOREGROUND: Key<Color> = Key::new("gitDecoration.deletedResourceForeground");
// pub const GIT_DECORATION_UNTRACKED_RESOURCE_FOREGROUND: Key<Color> =
//     Key::new("gitDecoration.untrackedResourceForeground");
// pub const GIT_DECORATION_CONFLICTING_RESOURCE_FOREGROUND: Key<Color> =
//     Key::new("gitDecoration.conflictingResourceForeground");
// pub const GIT_DECORATION_SUBMODULE_RESOURCE_FOREGROUND: Key<Color> =
//     Key::new("gitDecoration.submoduleResourceForeground");
