/// PDF composition primitives: page sizes, pages, documents, export options,
/// and a high-level composer that assembles them.

// ─── PageSize ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PageSize {
    A4,
    Letter,
    Legal,
    Custom(f32, f32),
}

impl PageSize {
    /// Returns (width_mm, height_mm) for this page size.
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match self {
            PageSize::A4 => (210.0, 297.0),
            PageSize::Letter => (215.9, 279.4),
            PageSize::Legal => (215.9, 355.6),
            PageSize::Custom(w, h) => (*w, *h),
        }
    }

    /// True when height >= width (portrait orientation).
    pub fn is_portrait(&self) -> bool {
        let (w, h) = self.dimensions_mm();
        h >= w
    }
}

// ─── PdfPage ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfPage {
    pub page_number: u32,
    pub size: PageSize,
    pub content_blocks: u32,
}

impl PdfPage {
    /// Area of this page in square millimetres.
    pub fn area_mm2(&self) -> f32 {
        let (w, h) = self.size.dimensions_mm();
        w * h
    }

    /// Human-readable label: "Page N (WxHmm)".
    pub fn label(&self) -> String {
        let (w, h) = self.size.dimensions_mm();
        format!("Page {} ({}x{}mm)", self.page_number, w as u32, h as u32)
    }
}

// ─── PdfDocument ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfDocument {
    pub title: String,
    pub pages: Vec<PdfPage>,
}

impl PdfDocument {
    pub fn new(title: impl Into<String>) -> Self {
        PdfDocument {
            title: title.into(),
            pages: Vec::new(),
        }
    }

    pub fn add_page(&mut self, page: PdfPage) {
        self.pages.push(page);
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Sum of content_blocks across all pages.
    pub fn total_content_blocks(&self) -> u32 {
        self.pages.iter().map(|p| p.content_blocks).sum()
    }
}

// ─── PdfExportOptions ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfExportOptions {
    pub compress: bool,
    pub embed_fonts: bool,
    pub dpi: u32,
}

impl PdfExportOptions {
    pub fn default() -> Self {
        PdfExportOptions {
            compress: true,
            embed_fonts: true,
            dpi: 300,
        }
    }

    /// True when dpi >= 300 AND fonts are embedded.
    pub fn is_high_quality(&self) -> bool {
        self.dpi >= 300 && self.embed_fonts
    }

    pub fn quality_label(&self) -> &'static str {
        if self.is_high_quality() {
            "high"
        } else {
            "standard"
        }
    }
}

// ─── PdfComposer ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfComposer {
    pub document: PdfDocument,
    pub options: PdfExportOptions,
}

impl PdfComposer {
    pub fn new(title: impl Into<String>) -> Self {
        PdfComposer {
            document: PdfDocument::new(title),
            options: PdfExportOptions::default(),
        }
    }

    pub fn add_page(&mut self, page: PdfPage) {
        self.document.add_page(page);
    }

    /// Summary string: "{title}: {N} pages, {M} blocks, {quality} quality".
    pub fn export_summary(&self) -> String {
        format!(
            "{}: {} pages, {} blocks, {} quality",
            self.document.title,
            self.document.page_count(),
            self.document.total_content_blocks(),
            self.options.quality_label(),
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod pdf_compose_tests {
    use super::*;

    #[test]
    fn page_size_dimensions_mm_a4() {
        assert_eq!(PageSize::A4.dimensions_mm(), (210.0, 297.0));
    }

    #[test]
    fn page_size_is_portrait_custom_landscape() {
        // width > height → landscape → is_portrait false
        let landscape = PageSize::Custom(400.0, 200.0);
        assert!(!landscape.is_portrait());
        // height >= width → portrait → is_portrait true
        let portrait = PageSize::Custom(200.0, 300.0);
        assert!(portrait.is_portrait());
    }

    #[test]
    fn pdf_page_area_mm2() {
        let page = PdfPage {
            page_number: 1,
            size: PageSize::A4,
            content_blocks: 0,
        };
        let expected = 210.0_f32 * 297.0_f32;
        assert!((page.area_mm2() - expected).abs() < 0.01);
    }

    #[test]
    fn pdf_page_label_format() {
        let page = PdfPage {
            page_number: 3,
            size: PageSize::A4,
            content_blocks: 2,
        };
        assert_eq!(page.label(), "Page 3 (210x297mm)");
    }

    #[test]
    fn pdf_document_total_content_blocks() {
        let mut doc = PdfDocument::new("Report");
        doc.add_page(PdfPage { page_number: 1, size: PageSize::A4, content_blocks: 5 });
        doc.add_page(PdfPage { page_number: 2, size: PageSize::Letter, content_blocks: 3 });
        assert_eq!(doc.total_content_blocks(), 8);
    }

    #[test]
    fn pdf_export_options_default_values() {
        let opts = PdfExportOptions::default();
        assert!(opts.compress);
        assert!(opts.embed_fonts);
        assert_eq!(opts.dpi, 300);
    }

    #[test]
    fn pdf_export_options_is_high_quality_true_and_false() {
        let high = PdfExportOptions { compress: false, embed_fonts: true, dpi: 600 };
        assert!(high.is_high_quality());

        let low = PdfExportOptions { compress: true, embed_fonts: false, dpi: 72 };
        assert!(!low.is_high_quality());
    }

    #[test]
    fn pdf_export_options_quality_label() {
        let high = PdfExportOptions::default();
        assert_eq!(high.quality_label(), "high");

        let standard = PdfExportOptions { compress: true, embed_fonts: false, dpi: 150 };
        assert_eq!(standard.quality_label(), "standard");
    }

    #[test]
    fn pdf_composer_export_summary_format() {
        let mut composer = PdfComposer::new("Annual Report");
        composer.add_page(PdfPage { page_number: 1, size: PageSize::A4, content_blocks: 4 });
        composer.add_page(PdfPage { page_number: 2, size: PageSize::A4, content_blocks: 6 });
        let summary = composer.export_summary();
        assert_eq!(summary, "Annual Report: 2 pages, 10 blocks, high quality");
    }
}
