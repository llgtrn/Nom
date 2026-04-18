/// Document-compose demo: prose → PDF
///
/// PdfElement — individual content item on a page
/// PdfPage    — a single PDF page with content elements
/// PdfDocument — collection of pages with metadata
/// PdfComposer — assembles a PdfDocument from prose input

// ─── PdfElement ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PdfElement {
    Text(String),
    Heading(String),
    Image(String),
    PageBreak,
}

impl PdfElement {
    pub fn element_type(&self) -> &str {
        match self {
            PdfElement::Text(_) => "text",
            PdfElement::Heading(_) => "heading",
            PdfElement::Image(_) => "image",
            PdfElement::PageBreak => "page_break",
        }
    }
}

// ─── PdfPage ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfPage {
    pub page_number: u32,
    pub elements: Vec<PdfElement>,
}

impl PdfPage {
    pub fn new(page_number: u32) -> Self {
        PdfPage {
            page_number,
            elements: Vec::new(),
        }
    }

    pub fn add_element(&mut self, el: PdfElement) {
        self.elements.push(el);
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Returns true if any element on this page is a Heading.
    pub fn has_heading(&self) -> bool {
        self.elements
            .iter()
            .any(|e| matches!(e, PdfElement::Heading(_)))
    }
}

// ─── PdfDocument ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfDocument {
    pub title: String,
    pub author: String,
    pub pages: Vec<PdfPage>,
}

impl PdfDocument {
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        PdfDocument {
            title: title.into(),
            author: author.into(),
            pages: Vec::new(),
        }
    }

    pub fn add_page(&mut self, page: PdfPage) {
        self.pages.push(page);
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Sum of element counts across all pages.
    pub fn total_elements(&self) -> usize {
        self.pages.iter().map(|p| p.element_count()).sum()
    }

    /// Rough word estimate: total_elements * 50.
    pub fn word_estimate(&self) -> usize {
        self.total_elements() * 50
    }
}

// ─── PdfComposer ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfComposer {
    pub author: String,
}

impl PdfComposer {
    pub fn new(author: impl Into<String>) -> Self {
        PdfComposer {
            author: author.into(),
        }
    }

    /// Splits prose by "\n\n" paragraphs; each paragraph becomes a Text element.
    /// Pages are filled 5 paragraphs at a time (page 1 holds paras 1–5, page 2
    /// holds 6–10, etc.).  An empty prose string still produces one empty page.
    pub fn compose_from_prose(&self, title: &str, prose: &str) -> PdfDocument {
        let mut doc = PdfDocument::new(title, self.author.clone());

        let paragraphs: Vec<&str> = if prose.is_empty() {
            vec![]
        } else {
            prose.split("\n\n").filter(|s| !s.trim().is_empty()).collect()
        };

        if paragraphs.is_empty() {
            doc.add_page(PdfPage::new(1));
            return doc;
        }

        let mut page_number: u32 = 1;
        let mut current_page = PdfPage::new(page_number);

        for (i, para) in paragraphs.iter().enumerate() {
            // Every 5 paragraphs start a new page (after the first group).
            if i > 0 && i % 5 == 0 {
                doc.add_page(current_page);
                page_number += 1;
                current_page = PdfPage::new(page_number);
            }
            current_page.add_element(PdfElement::Text(para.to_string()));
        }

        doc.add_page(current_page);
        doc
    }

    /// Preview: how many pages compose_from_prose would create for the given prose.
    pub fn page_count_for(&self, prose: &str) -> usize {
        let paragraphs: Vec<&str> = if prose.is_empty() {
            vec![]
        } else {
            prose.split("\n\n").filter(|s| !s.trim().is_empty()).collect()
        };

        if paragraphs.is_empty() {
            return 1;
        }

        // ceiling division: paragraphs / 5
        (paragraphs.len() + 4) / 5
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod pdf_compose_tests {
    use super::*;

    #[test]
    fn pdf_element_element_type_text() {
        let el = PdfElement::Text("hello world".to_string());
        assert_eq!(el.element_type(), "text");
    }

    #[test]
    fn pdf_element_element_type_heading() {
        let el = PdfElement::Heading("Chapter 1".to_string());
        assert_eq!(el.element_type(), "heading");
    }

    #[test]
    fn pdf_page_add_and_count() {
        let mut page = PdfPage::new(1);
        assert_eq!(page.element_count(), 0);
        page.add_element(PdfElement::Text("first".to_string()));
        page.add_element(PdfElement::Image("cover.png".to_string()));
        assert_eq!(page.element_count(), 2);
    }

    #[test]
    fn pdf_page_has_heading_true() {
        let mut page = PdfPage::new(1);
        page.add_element(PdfElement::Text("intro".to_string()));
        page.add_element(PdfElement::Heading("Title".to_string()));
        assert!(page.has_heading());
    }

    #[test]
    fn pdf_page_has_heading_false() {
        let mut page = PdfPage::new(2);
        page.add_element(PdfElement::Text("body text".to_string()));
        page.add_element(PdfElement::PageBreak);
        assert!(!page.has_heading());
    }

    #[test]
    fn pdf_document_add_page_count() {
        let mut doc = PdfDocument::new("My Doc", "Alice");
        assert_eq!(doc.page_count(), 0);
        doc.add_page(PdfPage::new(1));
        doc.add_page(PdfPage::new(2));
        assert_eq!(doc.page_count(), 2);
    }

    #[test]
    fn pdf_document_total_elements() {
        let mut doc = PdfDocument::new("Report", "Bob");

        let mut p1 = PdfPage::new(1);
        p1.add_element(PdfElement::Heading("Intro".to_string()));
        p1.add_element(PdfElement::Text("Para 1".to_string()));

        let mut p2 = PdfPage::new(2);
        p2.add_element(PdfElement::Text("Para 2".to_string()));

        doc.add_page(p1);
        doc.add_page(p2);

        assert_eq!(doc.total_elements(), 3);
        assert_eq!(doc.word_estimate(), 150);
    }

    #[test]
    fn pdf_composer_compose_single_para() {
        let composer = PdfComposer::new("Carol");
        let doc = composer.compose_from_prose("Test", "Single paragraph with no breaks.");
        assert_eq!(doc.page_count(), 1);
        assert_eq!(doc.total_elements(), 1);
        assert_eq!(doc.title, "Test");
        assert_eq!(doc.author, "Carol");
        if let PdfElement::Text(ref t) = doc.pages[0].elements[0] {
            assert_eq!(t, "Single paragraph with no breaks.");
        } else {
            panic!("expected Text element");
        }
    }

    #[test]
    fn pdf_composer_compose_multi_page() {
        let composer = PdfComposer::new("Dave");
        // 11 paragraphs → page 1 (5), page 2 (5), page 3 (1)
        let paras: Vec<String> = (1..=11).map(|i| format!("Paragraph {i}")).collect();
        let prose = paras.join("\n\n");

        let doc = composer.compose_from_prose("Multi", &prose);
        assert_eq!(doc.page_count(), 3, "11 paragraphs must span 3 pages");
        assert_eq!(doc.total_elements(), 11);

        // page_count_for must match
        assert_eq!(composer.page_count_for(&prose), 3);
    }
}
