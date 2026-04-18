//! Rich text primitives — markup tags, spans, paragraphs, blocks, and serialization.

/// Inline markup tag applied to a span of text.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupTag {
    /// Bold weight.
    Bold,
    /// Italic style.
    Italic,
    /// Underline decoration.
    Underline,
    /// Inline code formatting.
    Code,
    /// Hyperlink with a URL target.
    Link(String),
}

impl MarkupTag {
    /// Returns `true` for tags that affect visual appearance (Bold, Italic, Underline).
    pub fn is_visual(&self) -> bool {
        matches!(self, MarkupTag::Bold | MarkupTag::Italic | MarkupTag::Underline)
    }

    /// Returns the canonical lower-case name for the tag.
    pub fn tag_name(&self) -> &str {
        match self {
            MarkupTag::Bold => "bold",
            MarkupTag::Italic => "italic",
            MarkupTag::Underline => "underline",
            MarkupTag::Code => "code",
            MarkupTag::Link(_) => "link",
        }
    }
}

/// A run of text with zero or more markup tags applied.
#[derive(Debug, Clone)]
pub struct RichSpan {
    /// The raw text content of this span.
    pub text: String,
    /// Markup tags applied to this span.
    pub tags: Vec<MarkupTag>,
}

impl RichSpan {
    /// Returns `true` when no markup tags are applied.
    pub fn is_plain(&self) -> bool {
        self.tags.is_empty()
    }

    /// Returns `true` when any tag's `tag_name` equals `name`.
    pub fn has_tag(&self, name: &str) -> bool {
        self.tags.iter().any(|t| t.tag_name() == name)
    }

    /// Renders the span.  Plain spans emit their text as-is; tagged spans
    /// emit `[<first-tag-name>]<text>`.
    pub fn render(&self) -> String {
        if self.is_plain() {
            self.text.clone()
        } else {
            format!("[{}]{}", self.tags[0].tag_name(), self.text)
        }
    }
}

/// A paragraph composed of one or more `RichSpan`s with an optional indent level.
#[derive(Debug, Clone)]
pub struct RichParagraph {
    /// Ordered spans that make up this paragraph.
    pub spans: Vec<RichSpan>,
    /// Indent depth (0 = no indent).
    pub indent: u32,
}

impl RichParagraph {
    /// Returns the concatenated plain text of all spans (tags stripped).
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Returns the total number of characters across all spans.
    pub fn char_count(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }

    /// Returns `true` when the paragraph contains no characters.
    pub fn is_empty(&self) -> bool {
        self.char_count() == 0
    }
}

/// A rich-text block composed of one or more `RichParagraph`s.
#[derive(Debug, Clone, Default)]
pub struct RichTextBlock {
    /// Ordered paragraphs in this block.
    pub paragraphs: Vec<RichParagraph>,
}

impl RichTextBlock {
    /// Creates an empty `RichTextBlock`.
    pub fn new() -> Self {
        Self { paragraphs: Vec::new() }
    }

    /// Appends a paragraph to the block.
    pub fn add_paragraph(&mut self, p: RichParagraph) {
        self.paragraphs.push(p);
    }

    /// Returns the number of paragraphs in this block.
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs.len()
    }

    /// Returns the total character count across all paragraphs.
    pub fn total_chars(&self) -> usize {
        self.paragraphs.iter().map(|p| p.char_count()).sum()
    }

    /// Returns references to paragraphs that contain at least one character.
    pub fn non_empty_paragraphs(&self) -> Vec<&RichParagraph> {
        self.paragraphs.iter().filter(|p| !p.is_empty()).collect()
    }
}

/// Stateless serializer for `RichTextBlock`.
pub struct RichTextSerializer;

impl RichTextSerializer {
    /// Joins all paragraph plain texts with `"\n"`.
    pub fn to_plain(block: &RichTextBlock) -> String {
        block
            .paragraphs
            .iter()
            .map(|p| p.plain_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Counts words in the plain-text representation (split on whitespace, skip empty tokens).
    pub fn word_count(block: &RichTextBlock) -> usize {
        Self::to_plain(block)
            .split_whitespace()
            .filter(|w| !w.is_empty())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. MarkupTag::is_visual — visual tags return true, Code returns false
    #[test]
    fn markup_tag_is_visual() {
        assert!(MarkupTag::Bold.is_visual());
        assert!(MarkupTag::Italic.is_visual());
        assert!(MarkupTag::Underline.is_visual());
        assert!(!MarkupTag::Code.is_visual());
        assert!(!MarkupTag::Link("https://example.com".into()).is_visual());
    }

    // 2. tag_name for Link variant
    #[test]
    fn markup_tag_link_tag_name() {
        let tag = MarkupTag::Link("https://nom-lang.org".into());
        assert_eq!(tag.tag_name(), "link");
    }

    // 3. RichSpan::is_plain — no tags → plain
    #[test]
    fn rich_span_is_plain_when_no_tags() {
        let span = RichSpan { text: "hello".into(), tags: vec![] };
        assert!(span.is_plain());
    }

    // 4. RichSpan::has_tag matches by tag_name
    #[test]
    fn rich_span_has_tag_by_name() {
        let span = RichSpan {
            text: "world".into(),
            tags: vec![MarkupTag::Bold, MarkupTag::Link("http://x.com".into())],
        };
        assert!(span.has_tag("bold"));
        assert!(span.has_tag("link"));
        assert!(!span.has_tag("italic"));
    }

    // 5. RichSpan::render — plain span returns text as-is
    #[test]
    fn rich_span_render_plain() {
        let span = RichSpan { text: "plain text".into(), tags: vec![] };
        assert_eq!(span.render(), "plain text");
    }

    // 6. RichSpan::render — tagged span uses first tag name as prefix
    #[test]
    fn rich_span_render_with_tag() {
        let span = RichSpan {
            text: "emphasized".into(),
            tags: vec![MarkupTag::Italic],
        };
        assert_eq!(span.render(), "[italic]emphasized");
    }

    // 7. RichParagraph::plain_text joins span texts
    #[test]
    fn rich_paragraph_plain_text_join() {
        let para = RichParagraph {
            spans: vec![
                RichSpan { text: "Hello, ".into(), tags: vec![] },
                RichSpan { text: "world".into(), tags: vec![MarkupTag::Bold] },
                RichSpan { text: "!".into(), tags: vec![] },
            ],
            indent: 0,
        };
        assert_eq!(para.plain_text(), "Hello, world!");
    }

    // 8. RichParagraph::char_count sums span text lengths
    #[test]
    fn rich_paragraph_char_count() {
        let para = RichParagraph {
            spans: vec![
                RichSpan { text: "abc".into(), tags: vec![] },
                RichSpan { text: "de".into(), tags: vec![] },
            ],
            indent: 0,
        };
        assert_eq!(para.char_count(), 5);
    }

    // 9. RichTextBlock::total_chars and RichTextSerializer::word_count
    #[test]
    fn rich_text_block_total_chars_and_word_count() {
        let mut block = RichTextBlock::new();
        block.add_paragraph(RichParagraph {
            spans: vec![RichSpan { text: "one two".into(), tags: vec![] }],
            indent: 0,
        });
        block.add_paragraph(RichParagraph {
            spans: vec![RichSpan { text: "three".into(), tags: vec![] }],
            indent: 0,
        });
        assert_eq!(block.total_chars(), 12); // "one two" (7) + "three" (5)
        assert_eq!(RichTextSerializer::word_count(&block), 3);
    }
}
