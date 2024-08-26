use std::io;

use hyphenation::{Language, Load, Standard};
use owo_colors::OwoColorize;
use textwrap::{Options, WordSplitter};

use crate::note::Inline;

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

    pub fn fmt_comment(&self, mut w: impl io::Write, comment: &Inline) -> io::Result<()> {
        let text = textwrap::fill(&comment.text, &self.options);
        writeln!(w, "{text}")?;
        Ok(())
    }

    pub fn fmt_definition(
        &self,
        mut w: impl io::Write,
        term: &str,
        definition: &str,
    ) -> io::Result<()> {
        let term = term.bold();
        let text = textwrap::fill(definition, &self.options);
        writeln!(w, "  {term}\n{text}")
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}
