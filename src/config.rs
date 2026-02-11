use colored::Color;
use std::io::IsTerminal;

/// Theme colors for gg output
pub struct Theme {
    pub staged: Color,
    pub modified: Color,
    pub untracked: Color,
    pub deleted: Color,
    pub branch: Color,
    pub command: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            staged: Color::Green,
            modified: Color::Yellow,
            untracked: Color::Red,
            deleted: Color::Red,
            branch: Color::Cyan,
            command: Color::White,
        }
    }
}

/// Check if colors should be enabled.
/// Respects NO_COLOR standard (https://no-color.org/) and TTY detection.
pub fn colors_enabled() -> bool {
    // Respect NO_COLOR standard
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check if stdout is a terminal
    std::io::stdout().is_terminal()
}

/// Set up color handling based on environment.
/// Call this early in main().
pub fn setup_colors() {
    if !colors_enabled() {
        colored::control::set_override(false);
    }
}
