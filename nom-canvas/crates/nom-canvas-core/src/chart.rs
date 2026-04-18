//! Chart primitives: types, data series, config, and chart composition.
//!
//! Mirrors the graphify reference pattern: each chart has series data,
//! axis config, and render options.

/// All supported chart kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum ChartType {
    /// Vertical or horizontal bar chart.
    Bar,
    /// Connected line chart.
    Line,
    /// XY scatter plot.
    Scatter,
    /// Radial pie chart.
    Pie,
    /// Filled area chart.
    Area,
    /// Frequency histogram.
    Histogram,
}

impl ChartType {
    /// Human-readable name for this chart kind.
    pub fn display_name(&self) -> &str {
        match self {
            ChartType::Bar => "Bar Chart",
            ChartType::Line => "Line Chart",
            ChartType::Scatter => "Scatter Plot",
            ChartType::Pie => "Pie Chart",
            ChartType::Area => "Area Chart",
            ChartType::Histogram => "Histogram",
        }
    }

    /// Returns `true` when multiple overlapping data series make sense.
    ///
    /// Bar, Line, Area, and Scatter support multiple series; Pie and
    /// Histogram do not.
    pub fn supports_multiple_series(&self) -> bool {
        matches!(
            self,
            ChartType::Bar | ChartType::Line | ChartType::Area | ChartType::Scatter
        )
    }

    /// Returns `true` for radial chart kinds (currently only [`ChartType::Pie`]).
    pub fn is_radial(&self) -> bool {
        matches!(self, ChartType::Pie)
    }
}

/// One data series for a chart.
#[derive(Debug, Clone)]
pub struct DataSeries {
    /// Display label for this series (used in legends and tooltips).
    pub label: String,
    /// Ordered data values.
    pub values: Vec<f64>,
    /// Optional CSS-style color string (e.g. `"#ff6384"`).
    pub color: Option<String>,
}

impl DataSeries {
    /// Creates a new series with `label` and `values`; `color` defaults to `None`.
    pub fn new(label: &str, values: Vec<f64>) -> Self {
        Self {
            label: label.to_owned(),
            values,
            color: None,
        }
    }

    /// Minimum value in the series, or `f64::INFINITY` when empty.
    pub fn min(&self) -> f64 {
        self.values
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
    }

    /// Maximum value in the series, or `f64::NEG_INFINITY` when empty.
    pub fn max(&self) -> f64 {
        self.values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Sum of all values in the series.
    pub fn sum(&self) -> f64 {
        self.values.iter().sum()
    }

    /// Arithmetic mean of the series, or `0.0` when empty.
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum() / self.values.len() as f64
        }
    }
}

/// Axis labels, dimensions, and appearance settings for a chart.
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// Chart title displayed above the plot area.
    pub title: String,
    /// Label for the horizontal axis.
    pub x_label: String,
    /// Label for the vertical axis.
    pub y_label: String,
    /// Canvas width in logical pixels.
    pub width: u32,
    /// Canvas height in logical pixels.
    pub height: u32,
    /// Whether to render a series legend.
    pub show_legend: bool,
}

impl ChartConfig {
    /// Creates a config with `title` and sensible defaults (800×600, legend on).
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_owned(),
            x_label: String::new(),
            y_label: String::new(),
            width: 800,
            height: 600,
            show_legend: true,
        }
    }

    /// Builder method: overrides width and height.
    pub fn with_dimensions(mut self, w: u32, h: u32) -> Self {
        self.width = w;
        self.height = h;
        self
    }
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self::new("")
    }
}

/// A fully configured chart ready to render.
pub struct Chart {
    /// Which chart kind to render.
    pub kind: ChartType,
    /// Axis and appearance configuration.
    pub config: ChartConfig,
    /// Data series attached to this chart.
    pub series: Vec<DataSeries>,
}

impl Chart {
    /// Creates an empty chart of the given `kind` with default config.
    pub fn new(kind: ChartType) -> Self {
        Self {
            kind,
            config: ChartConfig::default(),
            series: Vec::new(),
        }
    }

    /// Builder method: replaces the chart config.
    pub fn with_config(mut self, config: ChartConfig) -> Self {
        self.config = config;
        self
    }

    /// Appends a data series and returns `&mut Self` for chaining.
    pub fn add_series(&mut self, series: DataSeries) -> &mut Self {
        self.series.push(series);
        self
    }

    /// Number of series currently attached.
    pub fn series_count(&self) -> usize {
        self.series.len()
    }

    /// Total number of data points across all series.
    pub fn total_data_points(&self) -> usize {
        self.series.iter().map(|s| s.values.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_type_display_name() {
        assert_eq!(ChartType::Bar.display_name(), "Bar Chart");
        assert_eq!(ChartType::Line.display_name(), "Line Chart");
        assert_eq!(ChartType::Scatter.display_name(), "Scatter Plot");
        assert_eq!(ChartType::Pie.display_name(), "Pie Chart");
        assert_eq!(ChartType::Area.display_name(), "Area Chart");
        assert_eq!(ChartType::Histogram.display_name(), "Histogram");
    }

    #[test]
    fn chart_type_pie_is_radial() {
        assert!(ChartType::Pie.is_radial());
        assert!(!ChartType::Bar.is_radial());
        assert!(!ChartType::Line.is_radial());
        assert!(!ChartType::Histogram.is_radial());
    }

    #[test]
    fn chart_type_bar_supports_multiple_series() {
        assert!(ChartType::Bar.supports_multiple_series());
        assert!(ChartType::Line.supports_multiple_series());
        assert!(ChartType::Area.supports_multiple_series());
        assert!(ChartType::Scatter.supports_multiple_series());
        assert!(!ChartType::Pie.supports_multiple_series());
        assert!(!ChartType::Histogram.supports_multiple_series());
    }

    #[test]
    fn data_series_min_max() {
        let s = DataSeries::new("test", vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        assert_eq!(s.min(), 1.0);
        assert_eq!(s.max(), 5.0);
    }

    #[test]
    fn data_series_sum_mean() {
        let s = DataSeries::new("test", vec![2.0, 4.0, 6.0]);
        assert_eq!(s.sum(), 12.0);
        assert_eq!(s.mean(), 4.0);
    }

    #[test]
    fn chart_config_defaults() {
        let cfg = ChartConfig::new("My Chart");
        assert_eq!(cfg.title, "My Chart");
        assert_eq!(cfg.width, 800);
        assert_eq!(cfg.height, 600);
        assert!(cfg.show_legend);
        assert!(cfg.x_label.is_empty());
        assert!(cfg.y_label.is_empty());
    }

    #[test]
    fn chart_add_series() {
        let mut chart = Chart::new(ChartType::Line);
        chart.add_series(DataSeries::new("alpha", vec![1.0, 2.0, 3.0]));
        chart.add_series(DataSeries::new("beta", vec![4.0, 5.0]));
        assert_eq!(chart.series_count(), 2);
    }

    #[test]
    fn chart_total_data_points() {
        let mut chart = Chart::new(ChartType::Bar);
        chart.add_series(DataSeries::new("a", vec![1.0, 2.0, 3.0]));
        chart.add_series(DataSeries::new("b", vec![10.0, 20.0]));
        assert_eq!(chart.total_data_points(), 5);
    }
}
