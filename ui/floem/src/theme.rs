use floem::peniko::Color;
use ndoc::theme::THEME;

pub struct Theme {
    pub editor_foreground: Color,
    pub editor_background: Color,
    pub selection_border: Color,
    pub selection_background: Color,
    pub palette_background: Color,
    pub palette_foreground: Color,
    pub syntect_theme: ndoc::SyntectTheme,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::from_ndoc_theme(&THEME).expect("Invalid default theme")
    }
}

impl Theme {
    pub fn from_ndoc_theme(theme : &ndoc::theme::Theme) -> Option<Self> {

        let selection_border = color_art::Color::from_hex(&theme.vscode.colors.selection_background)
        .unwrap()
        .darken(0.1)
        .hex_full();

        Some(Theme {
            editor_foreground: Color::parse(&theme.vscode.colors.editor_foreground)?,
            editor_background: Color::parse(&theme.vscode.colors.editor_background)?,
            palette_foreground: Color::parse(&theme.vscode.colors.editor_foreground)?,
            palette_background: Color::parse(&theme.vscode.colors.editor_background)?,
            selection_border: Color::parse(&selection_border)?,
            selection_background: Color::parse(&theme.vscode.colors.selection_background)?,
            syntect_theme: theme.style.clone(),
        })
    }
}