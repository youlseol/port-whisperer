use colored::Colorize;
use figlet_rs::FIGfont;

/// Structured banner with separate sections for gradient rendering.
pub struct Banner {
    /// All rendered lines: PORT block, blank separator, WHISPERER block, optional subtitle
    pub all_lines: Vec<String>,
    /// Number of PORT art lines (before the blank separator)
    pub port_line_count: usize,
    /// Number of WHISPERER art lines (after the separator)
    pub whisperer_line_count: usize,
}

/// Builds the banner for "PORT WHISPERER" with an optional subtitle.
pub fn build(subtitle: Option<&str>) -> Banner {
    let font: FIGfont = match FIGfont::standard() {
        Ok(f) => f,
        Err(_) => return fallback_banner(subtitle),
    };

    let port_lines = render_word(&font, "PORT");
    let whisperer_lines = render_word(&font, "WHISPERER");

    let port_count = port_lines.len();
    let whisperer_count = whisperer_lines.len();

    let mut all_lines = Vec::new();
    all_lines.extend(port_lines);
    all_lines.push(String::new()); // blank separator
    all_lines.extend(whisperer_lines);
    if let Some(sub) = subtitle {
        all_lines.push(String::new());
        all_lines.push(sub.to_string());
    }

    Banner { all_lines, port_line_count: port_count, whisperer_line_count: whisperer_count }
}

/// Convenience wrapper used by the legacy `lines()` call (plain mode).
pub fn lines() -> Vec<String> {
    build(None).all_lines
}

/// Prints the banner to stdout with gradient styling (PORT = green, WHISPERER = cyan).
pub fn print_plain() {
    let banner = build(None);
    for (i, line) in banner.all_lines.iter().enumerate() {
        if i < banner.port_line_count {
            println!("{}", line.green().bold());
        } else if i > banner.port_line_count
            && i <= banner.port_line_count + banner.whisperer_line_count
        {
            println!("{}", line.cyan().bold());
        } else {
            println!("{}", line.dimmed());
        }
    }
    println!();
}

fn render_word(font: &FIGfont, word: &str) -> Vec<String> {
    let Some(figure) = font.convert(word) else {
        return Vec::new();
    };
    let rendered: String = format!("{figure}");
    let mut word_lines: Vec<String> = rendered
        .lines()
        .map(|l: &str| l.to_string())
        .collect();
    while word_lines.last().map(|l: &String| l.trim().is_empty()).unwrap_or(false) {
        word_lines.pop();
    }
    word_lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_produces_non_empty_banner() {
        let b = build(None);
        assert!(!b.all_lines.is_empty(), "banner should have lines");
        assert!(b.port_line_count > 0, "PORT block should have lines");
        assert!(b.whisperer_line_count > 0, "WHISPERER block should have lines");
    }

    #[test]
    fn build_with_subtitle_appends_extra_lines() {
        let without = build(None).all_lines.len();
        let with_sub = build(Some("just say the word")).all_lines.len();
        // subtitle adds a blank line + the subtitle itself
        assert_eq!(with_sub, without + 2);
    }

    #[test]
    fn banner_structure_separator_present() {
        let b = build(None);
        // The blank separator between PORT and WHISPERER blocks must exist
        let separator_idx = b.port_line_count;
        assert!(
            b.all_lines[separator_idx].trim().is_empty(),
            "separator line between PORT and WHISPERER must be blank"
        );
    }

    #[test]
    fn fallback_banner_has_correct_counts() {
        // Force the fallback by calling it directly
        let b = fallback_banner(None);
        assert_eq!(b.port_line_count, 5);
        assert_eq!(b.whisperer_line_count, 5);
        // total: 5 PORT + 1 blank + 5 WHISPERER = 11
        assert_eq!(b.all_lines.len(), 11);
    }
}

/// Minimal fallback used when the figlet font fails to load.
fn fallback_banner(subtitle: Option<&str>) -> Banner {
    let port_lines: Vec<String> = vec![
        " ____   ___  ____  _____ ".into(),
        "|  _ \\ / _ \\|  _ \\|_   _|".into(),
        "| |_) | | | | |_) | | |  ".into(),
        "|  __/| |_| |  _ <  | |  ".into(),
        "|_|    \\___/|_| \\_\\ |_|  ".into(),
    ];
    let whisperer_lines: Vec<String> = vec![
        "__        ___  ___ ____  ____  _____ ____  _____ ____  ".into(),
        "\\ \\      / / || |_ _/ ___||  _ \\| ____|  _ \\| ____|  _ \\ ".into(),
        " \\ \\ /\\ / /| |_| || \\___ \\| |_) |  _| | |_) |  _| | |_) |".into(),
        "  \\ V  V / |  _  || |___) |  __/| |___|  _ <| |___|  _ < ".into(),
        "   \\_/\\_/  |_| |_|___|____/|_|   |_____|_| \\_\\_____|_| \\_\\".into(),
    ];
    let port_count = port_lines.len();
    let whisperer_count = whisperer_lines.len();
    let mut all_lines = Vec::new();
    all_lines.extend(port_lines);
    all_lines.push(String::new());
    all_lines.extend(whisperer_lines);
    if let Some(sub) = subtitle {
        all_lines.push(String::new());
        all_lines.push(sub.to_string());
    }
    Banner { all_lines, port_line_count: port_count, whisperer_line_count: whisperer_count }
}
