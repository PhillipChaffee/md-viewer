//! What the home list says about a document: its title, how long it is,
//! and the opening of its first paragraph.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[cfg(test)]
mod tests;

/// Longest excerpt kept, in characters: a little more than the two lines
/// the home list shows, so the cut falls where the list elides anyway.
const EXCERPT_CHARS: usize = 220;

/// What the home list shows of a document besides its path. It is worked
/// out once, when the document is loaded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DocumentSummary {
    /// The first level-one heading. `None` when the document has none, and
    /// the list falls back to the file's name.
    pub(crate) title: Option<String>,
    /// The start of the first paragraph of body text, as plain text.
    pub(crate) excerpt: Option<String>,
    pub(crate) word_count: Option<u32>,
}

impl DocumentSummary {
    /// The summary of `content`, whose first level-one heading is
    /// `document_title`.
    pub(crate) fn of(content: &str, document_title: Option<&str>) -> Self {
        let words = content.split_whitespace().count();
        Self {
            title: document_title.map(str::to_owned),
            excerpt: excerpt(content),
            word_count: Some(u32::try_from(words).unwrap_or(u32::MAX)),
        }
    }
}

/// The plain text of the first paragraph of body text, cut to
/// [`EXCERPT_CHARS`]. Quotes, lists, tables and footnotes are passed over,
/// and so is a paragraph with no words in it, such as a row of badges.
fn excerpt(content: &str) -> Option<String> {
    let mut reader = ParagraphReader::default();
    Parser::new_ext(content, Options::all())
        .find_map(|event| reader.read(&event))
        .map(|paragraph| truncated(&paragraph))
}

/// Walks a document's events and hands back the first paragraph worth
/// quoting.
#[derive(Default)]
struct ParagraphReader {
    /// How many blocks that are not body text enclose the current event.
    aside_depth: usize,
    /// How many images enclose the current event: their alt text is not
    /// part of the paragraph.
    image_depth: usize,
    /// The text of the paragraph being read, while one is open.
    paragraph: Option<String>,
}

impl ParagraphReader {
    /// Take one event in. Returns a finished paragraph that has words.
    fn read(&mut self, event: &Event<'_>) -> Option<String> {
        match event {
            Event::Start(tag) => self.open(tag),
            Event::End(tag) => return self.close(*tag),
            Event::Text(text) | Event::Code(text) | Event::InlineMath(text) => self.push(text),
            Event::SoftBreak | Event::HardBreak => self.push(" "),
            _ => {}
        }
        None
    }

    fn open(&mut self, tag: &Tag<'_>) {
        match tag {
            Tag::Paragraph if self.aside_depth == 0 => self.paragraph = Some(String::new()),
            Tag::Image { .. } => self.image_depth += 1,
            tag if is_aside(tag) => self.aside_depth += 1,
            _ => {}
        }
    }

    fn close(&mut self, tag: TagEnd) -> Option<String> {
        match tag {
            TagEnd::Paragraph => {
                return self.paragraph.take().filter(|text| has_words(text));
            }
            TagEnd::Image => self.image_depth = self.image_depth.saturating_sub(1),
            TagEnd::BlockQuote(_)
            | TagEnd::List(_)
            | TagEnd::Table
            | TagEnd::FootnoteDefinition
            | TagEnd::DefinitionList => self.aside_depth = self.aside_depth.saturating_sub(1),
            _ => {}
        }
        None
    }

    fn push(&mut self, text: &str) {
        if self.image_depth > 0 {
            return;
        }
        if let Some(paragraph) = &mut self.paragraph {
            paragraph.push_str(text);
        }
    }
}

/// Whether `tag` opens a block whose paragraphs are not the body text.
const fn is_aside(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::BlockQuote(_)
            | Tag::List(_)
            | Tag::Table(_)
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
    )
}

fn has_words(text: &str) -> bool {
    text.chars().any(char::is_alphanumeric)
}

/// `paragraph` on one line, cut at a word to at most [`EXCERPT_CHARS`]
/// characters, with an ellipsis where text was left out.
fn truncated(paragraph: &str) -> String {
    let mut excerpt = String::new();
    let mut length = 0;
    for word in paragraph.split_whitespace() {
        let word_length = word.chars().count();
        let separator = usize::from(length > 0);
        if length + separator + word_length > EXCERPT_CHARS {
            if length == 0 {
                // One unbroken run longer than the limit, as in a script
                // written without spaces: cut inside it.
                excerpt.extend(word.chars().take(EXCERPT_CHARS));
            }
            excerpt.push('…');
            return excerpt;
        }
        if separator == 1 {
            excerpt.push(' ');
        }
        excerpt.push_str(word);
        length += separator + word_length;
    }
    excerpt
}
