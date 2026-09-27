//! Line wrapping for annotation text, measured with Helvetica's AFM widths.
//!
//! The viewer paints annotation text in Helvetica and export writes the PDF standard
//! Helvetica font, so wrapping with one metric table puts line breaks in the same places.

pub const LINE_HEIGHT: f64 = 1.35;
/// Helvetica ascent, as a fraction of the font size.
pub const ASCENT: f64 = 0.718;

#[rustfmt::skip]
const ASCII_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // space-/
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 0-?
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // @-O
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // P-_
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // `-o
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,      // p-~
];

fn char_width(c: char) -> u16 {
    match c {
        ' '..='~' => ASCII_WIDTHS[c as usize - 32],
        'æ' => 889,
        'Æ' => 1000,
        'ø' | 'ß' => 611,
        'Ø' | 'Ö' | 'Ó' | 'Ò' | 'Ô' => 778,
        'Å' | 'Ä' | 'Á' | 'À' | 'Â' => 667,
        'É' | 'È' | 'Ê' | 'Ë' => 667,
        '–' => 556,
        '—' => 1000,
        '“' | '”' | '‘' | '’' => 333,
        '•' => 350,
        '…' => 1000,
        _ => 556,
    }
}

pub fn measure(text: &str, font_size: f64) -> f64 {
    text.chars().map(|c| char_width(c) as f64).sum::<f64>() * font_size / 1000.0
}

/// Breaks text into lines no wider than `max_width`, keeping explicit newlines.
/// A single word wider than the line stays on its own line rather than being split.
pub fn wrap(text: &str, font_size: f64, max_width: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.replace('\r', "").split('\n') {
        let mut current = String::new();
        for word in paragraph.split(' ') {
            let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
            if !current.is_empty() && measure(&candidate, font_size) > max_width {
                lines.push(std::mem::replace(&mut current, word.to_string()));
            } else {
                current = candidate;
            }
        }
        lines.push(current);
    }
    lines
}

/// Height of wrapped text laid out with [`LINE_HEIGHT`].
pub fn height(lines: usize, font_size: f64) -> f64 {
    lines.max(1) as f64 * font_size * LINE_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_on_width_and_newlines() {
        let lines = wrap("one two three\nfour", 10.0, measure("one two", 10.0) + 0.1);
        assert_eq!(lines, vec!["one two", "three", "four"]);
    }
}
