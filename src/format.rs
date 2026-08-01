use std::io;

use hyphenation::{Hyphenator, Language, Load, Standard};use owo_colors::OwoColorize;
use textwrap::termwidth;

use crate::note::{Kind, Note};

pub struct Formatter {
    width: usize,
    dictionary: Standard,
}

impl Formatter {
    pub fn new() -> Self {
        let dictionary = Standard::from_embedded(Language::EnglishUS).unwrap();
        Self {
            width: termwidth().min(80),
            dictionary,
        }
    }

    pub fn fmt_note(&self, mut w: impl io::Write, note: &Note) -> io::Result<()> {
        let loc = format!("{}:{}", note.path.display(), note.line);
        let header = match &note.kind {
            Kind::Define { term } => format!("{}  {}", term.bold(), loc.dimmed()),
            Kind::Fixme => format!("{}  {}", "FIXME".red().bold(), loc.dimmed()),
            Kind::Todo => format!("{}  {}", "TODO".yellow().bold(), loc.dimmed()),
            Kind::Note => loc.dimmed().to_string(),
        };
        writeln!(w, "{header}")?;
        self.write_body(&mut w, &note.text)?;
        Ok(())
    }

    pub fn fmt_notes(&self, mut w: impl io::Write, notes: &[&Note]) -> io::Result<()> {
        let mut first = true;
        for note in notes {
            if !first {
                writeln!(w)?;
            }
            first = false;
            self.fmt_note(&mut w, note)?;
        }
        Ok(())
    }

    pub fn fmt_glossary(&self, mut w: impl io::Write, notes: &[&Note]) -> io::Result<()> {
        let mut first = true;
        for note in notes {
            let Kind::Define { term } = &note.kind else {
                continue;
            };
            if !first {
                writeln!(w)?;
            }
            first = false;

            writeln!(w, "{}", term.bold())?;
            self.write_body(&mut w, &note.text)?;
        }
        Ok(())
    }

    fn write_body(&self, w: &mut impl io::Write, text: &str) -> io::Result<()> {
        let words = styled_words(text);
        let indent = "  ";
        let max = self.width.saturating_sub(indent.len()).max(1);

        for line in wrap_words(&words, max, &self.dictionary) {
            write_styled_line(w, indent, &line)?;
        }
        Ok(())
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Display style for a body span or word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyStyle {
    pub bold: bool,
    pub italic: bool,
    pub tag: bool,
}

/// A contiguous styled run in note body text (may contain spaces before word-split).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySpan {
    pub text: String,
    pub style: BodyStyle,
}

/// One whitespace-delimited word; may carry several styled segments so that, e.g.,
/// a tag name is underlined but its trailing punctuation stays plain (no gap).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyWord {
    pub segments: Vec<(String, BodyStyle)>,
}

impl BodyWord {
    /// Concatenated text of all segments.
    pub fn text(&self) -> String {
        self.segments.iter().map(|(t, _)| t.as_str()).collect()
    }

    /// Aggregated style (union of segment flags).
    #[allow(dead_code)]
    pub fn style_flags(&self) -> BodyStyle {
        let mut s = BodyStyle::default();
        for (_, st) in &self.segments {
            s.bold |= st.bold;
            s.italic |= st.italic;
            s.tag |= st.tag;
        }
        s
    }
}

/// Parse note text into styled spans: markdown emphasis + all `#tag` tokens as in-text tags.
pub fn body_for_display(text: &str) -> Vec<BodySpan> {
    parse_inline(text, BodyStyle::default())
}

/// Styled words for wrapping and TUI rendering. Whitespace delimits words; adjacent
/// non-whitespace runs of differing style become segments of the same word.
pub fn styled_words(text: &str) -> Vec<BodyWord> {
    let mut words: Vec<BodyWord> = Vec::new();
    let mut segments: Vec<(String, BodyStyle)> = Vec::new();
    let mut cur: Option<(String, BodyStyle)> = None;

    for span in body_for_display(text) {
        for c in span.text.chars() {
            if c.is_whitespace() {
                if let Some(s) = cur.take() {
                    segments.push(s);
                }
                if !segments.is_empty() {
                    words.push(BodyWord {
                        segments: std::mem::take(&mut segments),
                    });
                }
            } else {
                match &mut cur {
                    Some((t, s)) if *s == span.style => t.push(c),
                    _ => {
                        if let Some(s) = cur.take() {
                            segments.push(s);
                        }
                        cur = Some((c.to_string(), span.style));
                    }
                }
            }
        }
    }
    if let Some(s) = cur.take() {
        segments.push(s);
    }
    if !segments.is_empty() {
        words.push(BodyWord { segments });
    }
    words
}

/// Plain string for list/yank: markers and `#` removed.
pub fn plain_body(text: &str) -> String {
    styled_words(text)
        .iter()
        .map(|w| w.text())
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_styled_line(w: &mut impl io::Write, indent: &str, words: &[BodyWord]) -> io::Result<()> {
    write!(w, "{indent}")?;
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            write!(w, " ")?;
        }
        for (text, style) in &word.segments {
            write!(w, "{}", paint_word(text, *style))?;
        }
    }
    writeln!(w)
}

/// Greedy word wrap with end-of-line hyphenation. Returns words per line; each
/// word may carry multiple styled segments (e.g. tag name + plain punctuation).
fn wrap_words(words: &[BodyWord], max: usize, dict: &Standard) -> Vec<Vec<BodyWord>> {
    use std::collections::VecDeque;

    let mut queue: VecDeque<BodyWord> = words.iter().cloned().collect();
    let mut out: Vec<Vec<BodyWord>> = Vec::new();
    let mut line: Vec<BodyWord> = Vec::new();
    let mut line_len = 0usize;

    while let Some(word) = queue.front().cloned() {
        let wlen = word.text().chars().count();
        let need = if line.is_empty() { wlen } else { wlen + 1 };

        if line_len + need <= max {
            line.push(word);
            line_len += need;
            queue.pop_front();
            continue;
        }

        if !line.is_empty() {
            out.push(std::mem::take(&mut line));
            line_len = 0;
            continue;
        }

        // Hyphenate only single-segment words; multi-segment (e.g. tag + punct) are
        // short and left intact.
        let split = if word.segments.len() == 1 {
            let (t, s) = &word.segments[0];
            split_word(t, dict, max).map(|(head, tail)| (head, tail, *s))
        } else {
            None
        };

        match split {
            Some((head, tail, style)) => {
                line.push(BodyWord {
                    segments: vec![(format!("{head}-"), style)],
                });
                out.push(std::mem::take(&mut line));
                queue.pop_front();
                queue.push_front(BodyWord {
                    segments: vec![(tail, style)],
                });
            }
            None => {
                line.push(word);
                out.push(std::mem::take(&mut line));
                queue.pop_front();
            }
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// Break `word` so the head fits in `avail` columns (counting a trailing hyphen).
/// Returns `(head, tail)` at the largest valid hyphenation opportunity.
fn split_word(word: &str, dict: &Standard, avail: usize) -> Option<(String, String)> {
    if avail < 3 || word.chars().count() < 4 {
        return None;
    }
    let max_head = avail - 1;
    let breaks = dict.hyphenate(word).breaks;
    let mut best: Option<usize> = None;
    for &opp in &breaks {
        if opp == 0 || opp >= word.len() {
            continue;
        }
        let head_chars = word[..opp].chars().count();
        let tail_chars = word[opp..].chars().count();
        if head_chars <= max_head && tail_chars >= 2 {
            best = Some(opp);
        }
    }
    let opp = best?;
    Some((word[..opp].to_string(), word[opp..].to_string()))
}

fn parse_inline(input: &str, base: BodyStyle) -> Vec<BodySpan> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < input.len() {
        if bytes[i] == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            let end = scan_tag_end(input, i + 1);
            if end > i + 1 {
                let raw = &input[i + 1..end];
                let (name, punct) = tag_parts(raw);
                if !name.is_empty() {
                    push_span(
                        &mut out,
                        name.to_string(),
                        BodyStyle {
                            tag: true,
                            ..base
                        },
                    );
                    if !punct.is_empty() {
                        push_span(&mut out, punct.to_string(), base);
                    }
                    i = end;
                    continue;
                }
            }
        }

        if let Some((delim_len, close, style)) = match_open_delim(input, i) {
            if let Some(close_at) = find_close(input, i + delim_len, close) {
                let inner = &input[i + delim_len..close_at];
                if !inner.is_empty() {
                    let mut child = base;
                    child.bold |= style.bold;
                    child.italic |= style.italic;
                    out.extend(parse_inline(inner, child));
                    i = close_at + close.len();
                    continue;
                }
            }
            push_span(&mut out, input[i..i + delim_len].to_string(), base);
            i += delim_len;
            continue;
        }

        let next = next_special(input, i + 1);
        let end = next.unwrap_or(input.len());
        push_span(&mut out, input[i..end].to_string(), base);
        i = end;
    }

    out
}

fn match_open_delim(input: &str, i: usize) -> Option<(usize, &'static str, BodyStyle)> {
    let rest = &input[i..];
    if rest.starts_with("**") {
        return Some((
            2,
            "**",
            BodyStyle {
                bold: true,
                ..Default::default()
            },
        ));
    }
    if rest.starts_with("__") {
        return Some((
            2,
            "__",
            BodyStyle {
                bold: true,
                ..Default::default()
            },
        ));
    }
    if rest.starts_with('*') {
        return Some((
            1,
            "*",
            BodyStyle {
                italic: true,
                ..Default::default()
            },
        ));
    }
    if rest.starts_with('_') {
        return Some((
            1,
            "_",
            BodyStyle {
                italic: true,
                ..Default::default()
            },
        ));
    }
    None
}

fn find_close(input: &str, from: usize, close: &str) -> Option<usize> {
    input[from..].find(close).map(|rel| from + rel)
}

fn next_special(input: &str, from: usize) -> Option<usize> {
    input[from..]
        .find(['*', '_', '#'])
        .map(|rel| from + rel)
}

fn scan_tag_end(input: &str, from: usize) -> usize {
    let mut end = from;
    for (idx, c) in input[from..].char_indices() {
        if c.is_whitespace() {
            break;
        }
        end = from + idx + c.len_utf8();
    }
    end
}

fn push_span(out: &mut Vec<BodySpan>, text: String, style: BodyStyle) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.style == style {
            last.text.push_str(&text);
            return;
        }
    }
    out.push(BodySpan { text, style });
}

fn paint_word(w: &str, style: BodyStyle) -> String {
    match (style.bold, style.italic, style.tag) {
        (true, true, true) => w.bold().italic().underline().to_string(),
        (true, true, false) => w.bold().italic().to_string(),
        (true, false, true) => w.bold().underline().to_string(),
        (true, false, false) => w.bold().to_string(),
        (false, true, true) => w.italic().underline().to_string(),
        (false, true, false) => w.italic().to_string(),
        (false, false, true) => w.underline().to_string(),
        (false, false, false) => w.to_string(),
    }
}

fn tag_parts(rest: &str) -> (&str, &str) {
    let end = rest
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_ascii_alphanumeric())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    if end == 0 {
        (rest, "")
    } else {
        (&rest[..end], &rest[end..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_tags_are_kept_as_tags() {
        let words = styled_words("A sample tagged note. #tags #trailing");
        assert_eq!(
            plain_body("A sample tagged note. #tags #trailing"),
            "A sample tagged note. tags trailing"
        );
        assert!(words.iter().filter(|w| w.style_flags().tag).count() == 2);
    }

    #[test]
    fn keeps_in_text_tags() {
        let words = styled_words("A #sample tagged note.");
        assert_eq!(plain_body("A #sample tagged note."), "A sample tagged note.");
        assert!(words[1].style_flags().tag);
        assert_eq!(words[1].text(), "sample");
    }

    #[test]
    fn mixed_in_text_and_trailing() {
        let words = styled_words("See the #character arc. #plot #meta");
        assert_eq!(
            plain_body("See the #character arc. #plot #meta"),
            "See the character arc. plot meta"
        );
        assert!(words.iter().filter(|w| w.style_flags().tag).count() == 3);
    }

    #[test]
    fn sentence_final_tag_is_kept() {
        let words = styled_words("We learn that Death is his mother, #Athrune.");
        assert_eq!(
            plain_body("We learn that Death is his mother, #Athrune."),
            "We learn that Death is his mother, Athrune."
        );
        let athrune = words.iter().find(|w| w.text() == "Athrune.").unwrap();
        assert!(athrune.style_flags().tag);
        // The name is the tag; the trailing period is a separate plain segment.
        assert_eq!(athrune.segments.len(), 2);
        assert!(athrune.segments[0].1.tag);
        assert!(!athrune.segments[1].1.tag);
        assert_eq!(athrune.segments[1].0, ".");
    }

    #[test]
    fn all_tags_body_not_empty() {
        let words = styled_words("#only #tags");
        assert_eq!(plain_body("#only #tags"), "only tags");
        assert!(words.iter().all(|w| w.style_flags().tag));
    }

    #[test]
    fn bold_and_italic() {
        let words = styled_words("a **bold** and *italic* word");
        assert_eq!(
            plain_body("a **bold** and *italic* word"),
            "a bold and italic word"
        );
        assert!(words
            .iter()
            .any(|w| w.text() == "bold" && w.style_flags().bold && !w.style_flags().italic));
        assert!(words
            .iter()
            .any(|w| w.text() == "italic" && w.style_flags().italic && !w.style_flags().bold));
    }

    #[test]
    fn underscore_delimiters() {
        let plain = plain_body("use __bold__ and _italic_ here");
        assert_eq!(plain, "use bold and italic here");
    }

    #[test]
    fn nested_emphasis() {
        let words = styled_words("**bold *and* more**");
        assert_eq!(plain_body("**bold *and* more**"), "bold and more");
        let and = words.iter().find(|w| w.text() == "and").unwrap();
        assert!(and.style_flags().bold && and.style_flags().italic);
        let bold = words.iter().find(|w| w.text() == "bold").unwrap();
        assert!(bold.style_flags().bold && !bold.style_flags().italic);
    }

    #[test]
    fn unclosed_markers_stay_literal() {
        assert_eq!(plain_body("a *broken marker"), "a *broken marker");
        assert_eq!(plain_body("a **broken"), "a **broken");
    }

    #[test]
    fn multiword_emphasis() {
        assert_eq!(plain_body("say *hello world* now"), "say hello world now");
        let words = styled_words("say *hello world* now");
        assert!(words[1].style_flags().italic);
        assert!(words[2].style_flags().italic);
    }

    #[test]
    fn split_word_fits_largest_prefix() {
        let dict = Standard::from_embedded(Language::EnglishUS).unwrap();
        // "hyphenation" breaks at several points; with avail 6 we want the
        // largest head (5 chars + hyphen) that the dictionary permits.
        let (head, tail) = split_word("hyphenation", &dict, 6).expect("should split");
        assert!(head.chars().count() <= 5);
        assert!(tail.chars().count() >= 2);
        assert_eq!(format!("{head}{tail}"), "hyphenation");
    }

    #[test]
    fn split_word_skips_short_words() {
        let dict = Standard::from_embedded(Language::EnglishUS).unwrap();
        assert!(split_word("cat", &dict, 5).is_none());
    }

    #[test]
    fn wrap_hyphenates_overflowing_word() {
        let dict = Standard::from_embedded(Language::EnglishUS).unwrap();
        let words = styled_words("a wordy antidisestablishmentarianism ends here");
        let lines = wrap_words(&words, 14, &dict);

        // Each rendered line fits the width.
        for line in &lines {
            let rendered: String = line.iter().map(|w| w.text()).collect::<Vec<_>>().join(" ");
            assert!(
                rendered.chars().count() <= 14,
                "line too wide: {rendered:?}"
            );
        }

        // At least one break carries a hyphen for the long word.
        assert!(
            lines
                .iter()
                .flatten()
                .any(|w| w.text().ends_with('-')),
            "expected a hyphenated break"
        );

        // Reconstruct: a trailing hyphen means the word continues without a space.
        let mut rejoined = String::new();
        let mut prev_hyph = false;
        for w in lines.iter().flatten() {
            let text = w.text();
            let hyph = text.ends_with('-');
            let core = text.trim_end_matches('-');
            if prev_hyph {
                rejoined.push_str(core);
            } else {
                if !rejoined.is_empty() {
                    rejoined.push(' ');
                }
                rejoined.push_str(core);
            }
            prev_hyph = hyph;
        }
        assert_eq!(rejoined, "a wordy antidisestablishmentarianism ends here");
    }
}
