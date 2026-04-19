/// Chart type variants supported by the graphify composer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartType {
    Line,
    Bar,
    Scatter,
    Pie,
    Area,
}

impl ChartType {
    /// Returns a human-readable name for the chart type.
    pub fn chart_type_name(&self) -> &str {
        match self {
            ChartType::Line => "line",
            ChartType::Bar => "bar",
            ChartType::Scatter => "scatter",
            ChartType::Pie => "pie",
            ChartType::Area => "area",
        }
    }

    /// Returns true if this chart type supports multiple data series.
    /// All types support multiple series except Pie.
    pub fn supports_multiple_series(&self) -> bool {
        !matches!(self, ChartType::Pie)
    }
}

/// Axis label and unit for a chart axis.
#[derive(Debug, Clone)]
pub struct ChartAxis {
    pub label: String,
    pub unit: String,
}

impl ChartAxis {
    /// Creates a new axis with the given label and unit.
    pub fn new(label: impl Into<String>, unit: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            unit: unit.into(),
        }
    }

    /// Returns a display string: "Label (unit)" if unit is non-empty, else just "Label".
    pub fn display(&self) -> String {
        if self.unit.is_empty() {
            self.label.clone()
        } else {
            format!("{} ({})", self.label, self.unit)
        }
    }
}

/// A single data series within a chart.
#[derive(Debug, Clone)]
pub struct ChartSeries {
    pub name: String,
    pub data: Vec<f64>,
    pub color: String,
}

impl ChartSeries {
    /// Creates a new series with the given name and no data points.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            color: String::new(),
        }
    }

    /// Appends a data point to the series.
    pub fn add_point(&mut self, v: f64) {
        self.data.push(v);
    }

    /// Returns the number of data points in the series.
    pub fn point_count(&self) -> usize {
        self.data.len()
    }

    /// Returns the minimum value in the series, or f64::MAX if empty.
    pub fn min(&self) -> f64 {
        self.data.iter().cloned().fold(f64::MAX, f64::min)
    }

    /// Returns the maximum value in the series, or f64::MIN if empty.
    pub fn max(&self) -> f64 {
        self.data.iter().cloned().fold(f64::MIN, f64::max)
    }

    /// Returns the mean of the series, or 0.0 if empty.
    pub fn mean(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        self.data.iter().sum::<f64>() / self.data.len() as f64
    }
}

/// Specification for a complete chart: type, title, axes, and series collection.
#[derive(Debug, Clone)]
pub struct ChartSpec {
    pub title: String,
    pub chart_type: ChartType,
    pub x_axis: ChartAxis,
    pub y_axis: ChartAxis,
    pub series: Vec<ChartSeries>,
}

impl ChartSpec {
    /// Creates a new chart spec with no series.
    pub fn new(
        title: impl Into<String>,
        chart_type: ChartType,
        x_axis: ChartAxis,
        y_axis: ChartAxis,
    ) -> Self {
        Self {
            title: title.into(),
            chart_type,
            x_axis,
            y_axis,
            series: Vec::new(),
        }
    }

    /// Adds a data series to this chart spec.
    pub fn add_series(&mut self, s: ChartSeries) {
        self.series.push(s);
    }

    /// Returns the number of series in this chart spec.
    pub fn series_count(&self) -> usize {
        self.series.len()
    }

    /// Returns the total number of data points across all series.
    pub fn total_points(&self) -> usize {
        self.series.iter().map(|s| s.point_count()).sum()
    }
}

/// Composes chart specs from high-level parameters.
pub struct GraphifyComposer;

impl GraphifyComposer {
    /// Creates a new composer instance.
    pub fn new() -> Self {
        Self
    }

    /// Builds a Line chart spec with the given title and axis labels.
    pub fn compose_line(&self, title: &str, x_label: &str, y_label: &str) -> ChartSpec {
        ChartSpec::new(
            title,
            ChartType::Line,
            ChartAxis::new(x_label, ""),
            ChartAxis::new(y_label, ""),
        )
    }

    /// Builds a Bar chart spec with the given title and default axis labels.
    pub fn compose_bar(&self, title: &str) -> ChartSpec {
        ChartSpec::new(
            title,
            ChartType::Bar,
            ChartAxis::new("Category", ""),
            ChartAxis::new("Value", ""),
        )
    }

    /// Returns true for all chart types (all are supported).
    pub fn is_supported(&self, _chart_type: &ChartType) -> bool {
        true
    }
}

impl Default for GraphifyComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod graphify_chart_tests {
    use super::*;

    #[test]
    fn chart_type_name() {
        assert_eq!(ChartType::Line.chart_type_name(), "line");
        assert_eq!(ChartType::Bar.chart_type_name(), "bar");
        assert_eq!(ChartType::Scatter.chart_type_name(), "scatter");
        assert_eq!(ChartType::Pie.chart_type_name(), "pie");
        assert_eq!(ChartType::Area.chart_type_name(), "area");
    }

    #[test]
    fn chart_type_supports_multiple_series_pie_false() {
        assert!(!ChartType::Pie.supports_multiple_series());
        assert!(ChartType::Line.supports_multiple_series());
        assert!(ChartType::Bar.supports_multiple_series());
        assert!(ChartType::Scatter.supports_multiple_series());
        assert!(ChartType::Area.supports_multiple_series());
    }

    #[test]
    fn chart_axis_display_with_unit() {
        let axis = ChartAxis::new("Time", "ms");
        assert_eq!(axis.display(), "Time (ms)");
        let axis_no_unit = ChartAxis::new("Category", "");
        assert_eq!(axis_no_unit.display(), "Category");
    }

    #[test]
    fn chart_series_add_and_mean() {
        let mut s = ChartSeries::new("revenue");
        assert_eq!(s.mean(), 0.0);
        s.add_point(10.0);
        s.add_point(20.0);
        s.add_point(30.0);
        assert_eq!(s.mean(), 20.0);
        assert_eq!(s.point_count(), 3);
    }

    #[test]
    fn chart_series_min_max() {
        let mut s = ChartSeries::new("temps");
        // Empty series returns sentinel values
        assert_eq!(s.min(), f64::MAX);
        assert_eq!(s.max(), f64::MIN);
        s.add_point(5.0);
        s.add_point(-3.0);
        s.add_point(12.5);
        assert_eq!(s.min(), -3.0);
        assert_eq!(s.max(), 12.5);
    }

    #[test]
    fn chart_spec_add_series_count() {
        let mut spec = ChartSpec::new(
            "Sales",
            ChartType::Bar,
            ChartAxis::new("Month", ""),
            ChartAxis::new("Amount", "USD"),
        );
        assert_eq!(spec.series_count(), 0);
        spec.add_series(ChartSeries::new("Q1"));
        spec.add_series(ChartSeries::new("Q2"));
        assert_eq!(spec.series_count(), 2);
    }

    #[test]
    fn chart_spec_total_points() {
        let mut spec = ChartSpec::new(
            "Growth",
            ChartType::Line,
            ChartAxis::new("Year", ""),
            ChartAxis::new("Users", "k"),
        );
        let mut s1 = ChartSeries::new("organic");
        s1.add_point(1.0);
        s1.add_point(2.0);
        let mut s2 = ChartSeries::new("paid");
        s2.add_point(3.0);
        spec.add_series(s1);
        spec.add_series(s2);
        assert_eq!(spec.total_points(), 3);
    }

    #[test]
    fn graphify_composer_compose_line() {
        let composer = GraphifyComposer::new();
        let spec = composer.compose_line("Trend", "Time", "Value");
        assert_eq!(spec.title, "Trend");
        assert!(matches!(spec.chart_type, ChartType::Line));
        assert_eq!(spec.x_axis.label, "Time");
        assert_eq!(spec.y_axis.label, "Value");
        assert_eq!(spec.series_count(), 0);
    }

    #[test]
    fn graphify_composer_is_supported() {
        let composer = GraphifyComposer::new();
        assert!(composer.is_supported(&ChartType::Line));
        assert!(composer.is_supported(&ChartType::Bar));
        assert!(composer.is_supported(&ChartType::Scatter));
        assert!(composer.is_supported(&ChartType::Pie));
        assert!(composer.is_supported(&ChartType::Area));
    }
}
