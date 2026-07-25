use std::io;

use hyphenation::{Language, Load, Standard};
use owo_colors::OwoColorize;
use textwrap::{Options, WordSplitter};

use crate::note::{Kind, Note};

pub struct Formatter {
    options: Options<'static>,
}

impl Formatter {
    pub fn new() -> Self {
        let dictionary = Standard::from_embedded(Language::EnglishUS).unwrap();
        let options = Options::new(textwrap::termwidth().min(80))
            .initial_indent("  ")
            .subsequent_indent("  ")
            .word_splitter(WordSplitter::Hyphenation(dictionary));
        Self { options }
    }

    pub fn fmt_note(&self, mut w: impl io::Write, note: &Note) -> io::Result<()> {
        let loc = format!("{}:{}", note.path.display(), note.line);
        let header = match &note.kind {
            Kind::Define { term } => format!("{}  {}", term.bold(), loc.dimmed()),
            Kind::Fixme => format!("{}  {}", "FIXME".red().bold(), loc.dimmed()),
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

    /// Pretty-print a glossary of definitions (term + gloss).
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
        let body = body_for_display(text);
        let plain = plain_from_words(&body);
        let filled = textwrap::fill(&plain, &self.options);
        for line in filled.lines() {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            let content = &line[indent_len..];
            writeln!(w, "{indent}{}", style_plain_line(content, &body))?;
        }
        Ok(())
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

/// A word in display body text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyWord {
    /// Ordinary word
    Text(String),
    /// In-text tag (keep, style without `#`)
    Tag(String),
}

/// Split note text into display words: trailing `#tags` dropped; in-text tags kept.
pub fn body_for_display(text: &str) -> Vec<BodyWord> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut end = words.len();
    while end > 0 && is_tag_token(words[end - 1]) {
        end -= 1;
    }
    words[..end]
        .iter()
        .map(|w| {
            if let Some(rest) = w.strip_prefix('#') {
                let (name, punct) = tag_parts(rest);
                // punct stays as text after the tag name — attach via separate words if needed
                if punct.is_empty() {
                    BodyWord::Tag(name.to_string())
                } else {
                    // rare: #tag. mid-text — keep as tag name + punct in text form
                    BodyWord::Tag(format!("{name}{punct}"))
                }
            } else {
                BodyWord::Text((*w).to_string())
            }
        })
        .collect()
}

/// Plain string for list/yank: trailing tags stripped, in-text `#` removed.
pub fn plain_body(text: &str) -> String {
    plain_from_words(&body_for_display(text))
}

fn plain_from_words(words: &[BodyWord]) -> String {
    words
        .iter()
        .map(|w| match w {
            BodyWord::Text(s) | BodyWord::Tag(s) => s.as_str(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn style_plain_line(line: &str, body: &[BodyWord]) -> String {
    let tag_names: Vec<String> = body
        .iter()
        .filter_map(|w| match w {
            BodyWord::Tag(s) => {
                let (name, _) = tag_parts(s);
                Some(name.to_ascii_lowercase())
            }
            BodyWord::Text(_) => None,
        })
        .collect();

    line.split_whitespace()
        .map(|w| {
            let (core, punct) = tag_parts(w);
            if tag_names.iter().any(|t| t == &core.to_ascii_lowercase()) {
                format!("{}{punct}", core.underline())
            } else {
                w.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_tag_token(w: &str) -> bool {
    w.starts_with('#') && w.len() > 1
}

/// Split into (name, trailing punct).
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
    fn strips_trailing_tags_only() {
        let words = body_for_display("A sample tagged note. #tags #trailing");
        assert_eq!(
            plain_from_words(&words),
            "A sample tagged note."
        );
        assert!(words.iter().all(|w| matches!(w, BodyWord::Text(_))));
    }

    #[test]
    fn keeps_in_text_tags() {
        let words = body_for_display("A #sample tagged note.");
        assert_eq!(plain_from_words(&words), "A sample tagged note.");
        assert!(matches!(&words[1], BodyWord::Tag(s) if s == "sample"));
    }

    #[test]
    fn mixed_in_text_and_trailing() {
        let words = body_for_display("See the #character arc. #plot #meta");
        assert_eq!(plain_from_words(&words), "See the character arc.");
        assert!(matches!(&words[2], BodyWord::Tag(s) if s == "character"));
    }

    #[test]
    fn all_tags_means_empty_body() {
        let words = body_for_display("#only #tags");
        assert!(words.is_empty());
    }
}
