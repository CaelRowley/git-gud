use colored::Color;
use std::io::IsTerminal;

/// Theme colors for gg output
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default_colors() {
        let theme = Theme::default();
        assert_eq!(theme.staged, Color::Green);
        assert_eq!(theme.modified, Color::Yellow);
        assert_eq!(theme.untracked, Color::Red);
        assert_eq!(theme.deleted, Color::Red);
        assert_eq!(theme.branch, Color::Cyan);
    }

    #[test]
    fn test_colors_enabled_respects_no_color() {
        // Note: This test may be flaky depending on environment
        // In CI, NO_COLOR might be set
        let no_color_set = std::env::var("NO_COLOR").is_ok();
        if no_color_set {
            assert!(!colors_enabled());
        }
    }
}
