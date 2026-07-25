use std::path::{Path, PathBuf};

use regex::Regex;

use crate::note::{Kind, Note};

#[derive(Clone, Debug)]
pub struct Parser {
    comment: Regex,
    tags: Regex,
    define: Regex,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            comment: Regex::new(r"(?s)<!--\s*(.*?)\s*-->").unwrap(),
            tags: Regex::new(r"#\S+").unwrap(),
            define: Regex::new(
                r"(?i)^(?:def|define|definition)\s+(\S+)\s+(.+)$",
            )
            .unwrap(),
        }
    }

    pub fn parse_file(&self, path: &Path, source: &str) -> Vec<Note> {
        self.comment
            .captures_iter(source)
            .filter_map(|cx| {
                let full = cx.get(0)?;
                let body = cx.get(1)?.as_str().trim();
                let line = line_number(source, full.start());
                self.parse_body(path.to_path_buf(), line, body)
            })
            .collect()
    }

    fn parse_body(&self, path: PathBuf, line: usize, body: &str) -> Option<Note> {
        let (kind_label, rest) = split_keyword(body)?;
        let tags = extract_tags(&self.tags, body);

        let (kind, text) = if eq_ignore_ascii_case(kind_label, "FIXME") {
            (Kind::Fixme, rest.trim().to_string())
        } else if eq_ignore_ascii_case(kind_label, "NOTE") {
            if let Some(cx) = self.define.captures(rest.trim()) {
                let term = cx.get(1)?.as_str().to_ascii_lowercase();
                let gloss = cx.get(2)?.as_str().trim().to_string();
                (Kind::Define { term }, gloss)
            } else {
                (Kind::Note, rest.trim().to_string())
            }
        } else {
            return None;
        };

        if text.is_empty() && !matches!(kind, Kind::Define { .. }) {
            return None;
        }

        Some(Note {
            path,
            line,
            kind,
            tags,
            text,
        })
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

fn split_keyword(body: &str) -> Option<(&str, &str)> {
    let body = body.trim();
    let end = body
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, _)| i)
        .unwrap_or(body.len());
    if end == 0 {
        return None;
    }
    let keyword = &body[..end];
    let rest = body[end..].trim_start();
    Some((keyword, rest))
}

fn extract_tags(pattern: &Regex, text: &str) -> Vec<String> {
    pattern
        .captures_iter(text)
        .filter_map(|cx| {
            let raw = cx.get(0)?.as_str();
            let tag = raw
                .trim_start_matches('#')
                .trim_end_matches(|u: char| !u.is_ascii_alphanumeric())
                .to_ascii_lowercase();
            if tag.is_empty() {
                None
            } else {
                Some(tag)
            }
        })
        .collect()
}

fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn line_number(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].bytes().filter(|&b| b == b'\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_one(source: &str) -> Note {
        let notes = Parser::new().parse_file(Path::new("t.md"), source);
        assert_eq!(notes.len(), 1, "expected one note in {source:?}, got {notes:?}");
        notes.into_iter().next().unwrap()
    }

    #[test]
    fn parses_note_with_tags() {
        let n = parse_one("See Spot. <!-- NOTE Spot is a dog. #character #bio -->\n");
        assert_eq!(n.kind, Kind::Note);
        assert_eq!(n.text, "Spot is a dog. #character #bio");
        assert_eq!(n.tags, vec!["character", "bio"]);
        assert_eq!(n.line, 1);
    }

    #[test]
    fn parses_fixme() {
        let n = parse_one("<!-- FIXME timeline inconsistency #plot -->");
        assert_eq!(n.kind, Kind::Fixme);
        assert_eq!(n.tags, vec!["plot"]);
        assert!(n.text.contains("timeline"));
    }

    #[test]
    fn parses_define_def() {
        let n = parse_one(
            "<!-- NOTE def spearsheaves a tax taken directly from the crop. -->",
        );
        assert_eq!(
            n.kind,
            Kind::Define {
                term: "spearsheaves".into()
            }
        );
        assert_eq!(n.text, "a tax taken directly from the crop.");
    }

    #[test]
    fn parses_define_definition() {
        let n = parse_one("<!-- NOTE definition Foo bar baz -->");
        assert_eq!(n.kind, Kind::Define { term: "foo".into() });
        assert_eq!(n.text, "bar baz");
    }

    #[test]
    fn ignores_plain_html_comments() {
        let notes = Parser::new().parse_file(Path::new("t.md"), "<!-- just a comment -->");
        assert!(notes.is_empty());
    }

    #[test]
    fn tracks_line_numbers() {
        let source = "line1\nline2\n<!-- NOTE hello #x -->\n";
        let n = parse_one(source);
        assert_eq!(n.line, 3);
    }

    #[test]
    fn fixme_tags_extracted() {
        let n = parse_one("<!-- FIXME broken #errata #plot -->");
        assert!(n.is_fixme());
        assert_eq!(n.tags, vec!["errata", "plot"]);
    }
}
