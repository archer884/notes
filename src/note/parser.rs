use std::{collections::HashMap, fmt, str::FromStr};

use regex::Regex;

use crate::note::{Comment, Definition, Inline};

#[derive(Debug)]
pub enum ParseInlineError {
    InvalidAttribute(String),
    MissingAttribute(Attribute),
    UnknownType(String),
}

impl fmt::Display for ParseInlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAttribute(content) => {
                write!(f, "tag contains invalid attribute: {content}")
            }
            Self::MissingAttribute(attr) => write!(f, "expected attribute: {attr}"),
            Self::UnknownType(tag) => write!(f, "unknown tag type: {tag}"),
        }
    }
}

impl std::error::Error for ParseInlineError {}

#[derive(Clone, Debug)]
pub struct InlineParser {
    tag_parser: TagParser,
}

impl InlineParser {
    pub fn new() -> Self {
        Self {
            tag_parser: TagParser::new(),
        }
    }

    pub fn parse(&self, s: &str) -> Result<Inline, ParseInlineError> {
        let s = s.trim_start_matches("<note ").trim_end_matches('>');
        let attributes = self.tag_parser.build_attribute_dict(s)?;

        // Our inline was a comment. This means we require a set of tags and, optionally, will
        // store a heading to go along with the comment.
        if let Some(comment) = attributes.get(&Attribute::Comment) {
            let tags = attributes
                .get(&Attribute::Tags)
                .ok_or(ParseInlineError::MissingAttribute(Attribute::Tags))?
                .trim_matches('"')
                .split(',')
                .map(|s| s.into())
                .collect();

            return Ok(Inline::Comment(Box::new(Comment {
                tags,
                heading: attributes
                    .get(&Attribute::Heading)
                    .map(|s| s.trim_matches('"').into()),
                comment: comment.trim_matches('"').into(),
            })));
        }

        // Our inline was a term definition. This means we require both a term and a definition.
        // Now that I've written out the comment, that seems pretty obvious.
        if let Some(term) = attributes.get(&Attribute::Term) {
            let definition = attributes
                .get(&Attribute::Definition)
                .ok_or(ParseInlineError::MissingAttribute(Attribute::Definition))?
                .trim_matches('"');

            return Ok(Inline::Definition(Box::new(Definition {
                term: term.trim_matches('"').into(),
                definition: definition.into(),
            })));
        }

        Err(ParseInlineError::UnknownType(s.into()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attribute {
    Comment,
    Definition,
    Heading,
    Tags,
    Term,
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Attribute::Comment => f.write_str("Comment"),
            Attribute::Definition => f.write_str("Definition"),
            Attribute::Heading => f.write_str("Heading"),
            Attribute::Tags => f.write_str("Tags"),
            Attribute::Term => f.write_str("Term"),
        }
    }
}

impl FromStr for Attribute {
    type Err = ParseInlineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_ascii_lowercase() {
            "comment" => Ok(Attribute::Comment),
            "definition" => Ok(Attribute::Definition),
            "heading" => Ok(Attribute::Heading),
            "tag" | "tags" => Ok(Attribute::Tags),
            "term" => Ok(Attribute::Term),
            bad_attribute => Err(ParseInlineError::InvalidAttribute(bad_attribute.into())),
        }
    }
}

#[derive(Clone, Debug)]
struct TagParser {
    attribute_rx: Regex,
}

impl TagParser {
    fn new() -> TagParser {
        Self {
            attribute_rx: Regex::new("(?:(term|definition|heading|tags?|comment)=)").unwrap(),
        }
    }

    pub fn build_attribute_dict(
        &self,
        s: &str,
    ) -> Result<HashMap<Attribute, String>, ParseInlineError> {
        // The plan is to split on attribute keys and assume anything in between those is probably
        // meant to be the attribute value. This may allow us to ignore quotes inside quotes, which
        // is a very desirable property.

        let mut map = HashMap::new();
        let attributes = self.attribute_rx.find_iter(s).pairs().map(|(left, right)| {
            right
                .map(|right| {
                    (
                        &s[left.start()..left.end() - 1],
                        s[left.end()..right.start()].trim(),
                    )
                })
                .unwrap_or_else(|| (&s[left.start()..left.end() - 1], s[left.end()..].trim()))
        });

        for (key, value) in attributes {
            let attr: Attribute = key.parse()?;
            map.insert(attr, value.into());
        }

        Ok(map)
    }
}

struct PairsIter<I, T> {
    source: I,
    left: Option<T>,
}

impl<I, T> Iterator for PairsIter<I, T>
where
    I: Iterator<Item = T>,
    T: Copy,
{
    type Item = (T, Option<T>);

    fn next(&mut self) -> Option<Self::Item> {
        let left = self.left.take()?;
        self.source
            .next()
            .map(|right| {
                self.left = Some(right);
                Some((left, Some(right)))
            })
            .unwrap_or(Some((left, None)))
    }
}

trait Pairs: Iterator + Sized {
    fn pairs(self) -> PairsIter<Self, Self::Item>;
}

impl<T: Iterator> Pairs for T {
    fn pairs(mut self) -> PairsIter<Self, Self::Item> {
        PairsIter {
            left: self.next(),
            source: self,
        }
    }
}
