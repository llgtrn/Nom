//! Table block primitives — cells, rows, blocks, and CSV serialization.

/// Horizontal alignment for a table cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellAlign {
    /// Left-aligned (the default).
    Left,
    /// Center-aligned.
    Center,
    /// Right-aligned.
    Right,
}

impl CellAlign {
    /// Returns the CSS text-align value for this alignment.
    pub fn css_value(&self) -> &'static str {
        match self {
            CellAlign::Left => "left",
            CellAlign::Center => "center",
            CellAlign::Right => "right",
        }
    }

    /// Returns `true` only for `Left`, which is the default alignment.
    pub fn is_default(&self) -> bool {
        matches!(self, CellAlign::Left)
    }
}

/// A single cell inside a table row.
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Text content of the cell.
    pub content: String,
    /// Number of columns this cell spans (1 = normal).
    pub colspan: u8,
    /// Horizontal alignment of the cell content.
    pub align: CellAlign,
}

impl TableCell {
    /// Returns `true` when this cell spans more than one column.
    pub fn is_merged(&self) -> bool {
        self.colspan > 1
    }

    /// Returns a display string in the format `"[{css_value}/{colspan}] {content}"`.
    pub fn display(&self) -> String {
        format!("[{}/{}] {}", self.align.css_value(), self.colspan, self.content)
    }
}

/// A row of cells inside a `TableBlock`.
#[derive(Debug, Clone)]
pub struct TableRow {
    /// The cells that make up this row.
    pub cells: Vec<TableCell>,
    /// When `true` this row is rendered as a header row.
    pub is_header: bool,
}

impl TableRow {
    /// Returns the number of cells in the row (ignoring colspan).
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Returns the total column width occupied by this row (sum of each cell's colspan).
    pub fn effective_width(&self) -> usize {
        self.cells.iter().map(|c| c.colspan as usize).sum()
    }

    /// Returns `true` when at least one cell in this row spans multiple columns.
    pub fn has_merged_cells(&self) -> bool {
        self.cells.iter().any(|c| c.is_merged())
    }
}

/// A complete table composed of rows, with an optional caption.
#[derive(Debug, Clone)]
pub struct TableBlock {
    /// All rows in this table (headers and data rows mixed).
    pub rows: Vec<TableRow>,
    /// Optional caption displayed above or below the table.
    pub caption: Option<String>,
}

impl TableBlock {
    /// Creates an empty `TableBlock` with no rows and no caption.
    pub fn new() -> Self {
        TableBlock {
            rows: Vec::new(),
            caption: None,
        }
    }

    /// Appends a row to the end of the table.
    pub fn add_row(&mut self, row: TableRow) {
        self.rows.push(row);
    }

    /// Returns references to all header rows (`is_header == true`).
    pub fn header_rows(&self) -> Vec<&TableRow> {
        self.rows.iter().filter(|r| r.is_header).collect()
    }

    /// Returns references to all data rows (`is_header == false`).
    pub fn data_rows(&self) -> Vec<&TableRow> {
        self.rows.iter().filter(|r| !r.is_header).collect()
    }

    /// Returns the maximum effective width across all rows, or 0 when the table is empty.
    pub fn column_count(&self) -> usize {
        self.rows.iter().map(|r| r.effective_width()).max().unwrap_or(0)
    }
}

impl Default for TableBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializes a `TableBlock` to various text formats.
pub struct TableSerializer;

impl TableSerializer {
    /// Serializes only the data rows (non-header) to CSV.
    ///
    /// Each row is a comma-separated list of cell contents; rows are separated by `"\n"`.
    /// Header rows are skipped entirely.
    pub fn to_csv(table: &TableBlock) -> String {
        table
            .data_rows()
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|c| c.content.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns the total number of rows in the table (including header rows).
    pub fn row_count(table: &TableBlock) -> usize {
        table.rows.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. CellAlign::css_value returns the correct CSS string for each variant.
    #[test]
    fn cell_align_css_value() {
        assert_eq!(CellAlign::Left.css_value(), "left");
        assert_eq!(CellAlign::Center.css_value(), "center");
        assert_eq!(CellAlign::Right.css_value(), "right");
    }

    // 2. CellAlign::is_default is true only for Left.
    #[test]
    fn cell_align_is_default() {
        assert!(CellAlign::Left.is_default());
        assert!(!CellAlign::Center.is_default());
        assert!(!CellAlign::Right.is_default());
    }

    // 3. TableCell::is_merged returns true when colspan > 1.
    #[test]
    fn table_cell_is_merged() {
        let single = TableCell { content: "a".into(), colspan: 1, align: CellAlign::Left };
        let merged = TableCell { content: "b".into(), colspan: 3, align: CellAlign::Left };
        assert!(!single.is_merged());
        assert!(merged.is_merged());
    }

    // 4. TableCell::display produces the expected "[css/colspan] content" format.
    #[test]
    fn table_cell_display_format() {
        let cell = TableCell {
            content: "Hello".into(),
            colspan: 2,
            align: CellAlign::Center,
        };
        assert_eq!(cell.display(), "[center/2] Hello");
    }

    // 5. TableRow::effective_width sums colspans correctly.
    #[test]
    fn table_row_effective_width_with_colspan() {
        let row = TableRow {
            cells: vec![
                TableCell { content: "a".into(), colspan: 2, align: CellAlign::Left },
                TableCell { content: "b".into(), colspan: 1, align: CellAlign::Right },
                TableCell { content: "c".into(), colspan: 3, align: CellAlign::Center },
            ],
            is_header: false,
        };
        assert_eq!(row.effective_width(), 6);
    }

    // 6. TableRow::has_merged_cells returns true only when at least one cell spans > 1.
    #[test]
    fn table_row_has_merged_cells() {
        let no_merge = TableRow {
            cells: vec![
                TableCell { content: "x".into(), colspan: 1, align: CellAlign::Left },
                TableCell { content: "y".into(), colspan: 1, align: CellAlign::Left },
            ],
            is_header: false,
        };
        let with_merge = TableRow {
            cells: vec![
                TableCell { content: "x".into(), colspan: 1, align: CellAlign::Left },
                TableCell { content: "y".into(), colspan: 2, align: CellAlign::Left },
            ],
            is_header: false,
        };
        assert!(!no_merge.has_merged_cells());
        assert!(with_merge.has_merged_cells());
    }

    // 7. TableBlock::column_count returns the maximum effective_width across all rows (0 when empty).
    #[test]
    fn table_block_column_count() {
        let mut table = TableBlock::new();
        assert_eq!(table.column_count(), 0);

        table.add_row(TableRow {
            cells: vec![
                TableCell { content: "a".into(), colspan: 1, align: CellAlign::Left },
                TableCell { content: "b".into(), colspan: 2, align: CellAlign::Left },
            ],
            is_header: false,
        });
        table.add_row(TableRow {
            cells: vec![
                TableCell { content: "c".into(), colspan: 4, align: CellAlign::Left },
            ],
            is_header: false,
        });
        // max(3, 4) = 4
        assert_eq!(table.column_count(), 4);
    }

    // 8. TableBlock::header_rows vs data_rows correctly partition rows by is_header.
    #[test]
    fn table_block_header_rows_vs_data_rows() {
        let mut table = TableBlock::new();
        table.add_row(TableRow {
            cells: vec![TableCell { content: "Name".into(), colspan: 1, align: CellAlign::Left }],
            is_header: true,
        });
        table.add_row(TableRow {
            cells: vec![TableCell { content: "Alice".into(), colspan: 1, align: CellAlign::Left }],
            is_header: false,
        });
        table.add_row(TableRow {
            cells: vec![TableCell { content: "Bob".into(), colspan: 1, align: CellAlign::Left }],
            is_header: false,
        });

        assert_eq!(table.header_rows().len(), 1);
        assert_eq!(table.data_rows().len(), 2);
        assert_eq!(table.header_rows()[0].cells[0].content, "Name");
    }

    // 9. TableSerializer::to_csv skips header rows and serializes only data rows.
    #[test]
    fn table_serializer_to_csv_skips_headers() {
        let mut table = TableBlock::new();
        // Header row — must be absent from CSV output.
        table.add_row(TableRow {
            cells: vec![
                TableCell { content: "Name".into(), colspan: 1, align: CellAlign::Left },
                TableCell { content: "Age".into(), colspan: 1, align: CellAlign::Left },
            ],
            is_header: true,
        });
        // Data rows.
        table.add_row(TableRow {
            cells: vec![
                TableCell { content: "Alice".into(), colspan: 1, align: CellAlign::Left },
                TableCell { content: "30".into(), colspan: 1, align: CellAlign::Left },
            ],
            is_header: false,
        });
        table.add_row(TableRow {
            cells: vec![
                TableCell { content: "Bob".into(), colspan: 1, align: CellAlign::Left },
                TableCell { content: "25".into(), colspan: 1, align: CellAlign::Left },
            ],
            is_header: false,
        });

        let csv = TableSerializer::to_csv(&table);
        assert_eq!(csv, "Alice,30\nBob,25");
        // Total row count includes the header.
        assert_eq!(TableSerializer::row_count(&table), 3);
    }
}
