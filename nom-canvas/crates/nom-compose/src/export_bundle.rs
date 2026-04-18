#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Pdf,
    Mp4,
    Zip,
    Wasm,
}

impl ExportFormat {
    pub fn is_binary(&self) -> bool {
        matches!(self, ExportFormat::Pdf | ExportFormat::Mp4 | ExportFormat::Zip | ExportFormat::Wasm)
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Mp4 => "mp4",
            ExportFormat::Zip => "zip",
            ExportFormat::Wasm => "wasm",
        }
    }
}

pub struct ExportTarget {
    pub format: ExportFormat,
    pub output_path: String,
    pub include_assets: bool,
}

impl ExportTarget {
    pub fn filename(&self) -> String {
        match self.output_path.rfind('/') {
            Some(idx) => self.output_path[idx + 1..].to_string(),
            None => self.output_path.clone(),
        }
    }

    pub fn is_archive(&self) -> bool {
        self.format == ExportFormat::Zip
    }
}

pub struct ExportJob {
    pub id: u64,
    pub target: ExportTarget,
    pub status: String,
    pub size_bytes: Option<u64>,
}

impl ExportJob {
    pub fn is_complete(&self) -> bool {
        self.status == "done"
    }

    pub fn mark_done(&mut self, size: u64) {
        self.status = "done".to_string();
        self.size_bytes = Some(size);
    }
}

pub struct ExportQueue {
    pub jobs: Vec<ExportJob>,
}

impl ExportQueue {
    pub fn new() -> Self {
        ExportQueue { jobs: Vec::new() }
    }

    pub fn enqueue(&mut self, job: ExportJob) {
        self.jobs.push(job);
    }

    pub fn pending_count(&self) -> usize {
        self.jobs.iter().filter(|j| !j.is_complete()).count()
    }

    pub fn complete_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.is_complete()).count()
    }

    pub fn total_size_bytes(&self) -> u64 {
        self.jobs.iter().filter_map(|j| j.size_bytes).sum()
    }
}

pub struct ExportResult {
    pub job_id: u64,
    pub format: ExportFormat,
    pub bytes_written: u64,
}

impl ExportResult {
    pub fn is_large(&self) -> bool {
        self.bytes_written > 10_000_000
    }

    pub fn summary(&self) -> String {
        format!(
            "job:{} format:{} bytes:{}",
            self.job_id,
            self.format.file_extension(),
            self.bytes_written
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(format: ExportFormat, path: &str) -> ExportTarget {
        ExportTarget {
            format,
            output_path: path.to_string(),
            include_assets: false,
        }
    }

    fn make_job(id: u64, format: ExportFormat, path: &str, status: &str) -> ExportJob {
        ExportJob {
            id,
            target: make_target(format, path),
            status: status.to_string(),
            size_bytes: None,
        }
    }

    // Test 1: format is_binary
    #[test]
    fn test_format_is_binary() {
        assert!(!ExportFormat::Json.is_binary());
        assert!(!ExportFormat::Csv.is_binary());
        assert!(ExportFormat::Pdf.is_binary());
        assert!(ExportFormat::Mp4.is_binary());
        assert!(ExportFormat::Zip.is_binary());
        assert!(ExportFormat::Wasm.is_binary());
    }

    // Test 2: format file_extension
    #[test]
    fn test_format_file_extension() {
        assert_eq!(ExportFormat::Json.file_extension(), "json");
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Pdf.file_extension(), "pdf");
        assert_eq!(ExportFormat::Mp4.file_extension(), "mp4");
        assert_eq!(ExportFormat::Zip.file_extension(), "zip");
        assert_eq!(ExportFormat::Wasm.file_extension(), "wasm");
    }

    // Test 3: target filename
    #[test]
    fn test_target_filename() {
        let t1 = make_target(ExportFormat::Json, "output/data/result.json");
        assert_eq!(t1.filename(), "result.json");

        let t2 = make_target(ExportFormat::Csv, "report.csv");
        assert_eq!(t2.filename(), "report.csv");
    }

    // Test 4: target is_archive
    #[test]
    fn test_target_is_archive() {
        let zip = make_target(ExportFormat::Zip, "bundle.zip");
        assert!(zip.is_archive());

        let pdf = make_target(ExportFormat::Pdf, "doc.pdf");
        assert!(!pdf.is_archive());
    }

    // Test 5: job is_complete
    #[test]
    fn test_job_is_complete() {
        let done = make_job(1, ExportFormat::Json, "out.json", "done");
        assert!(done.is_complete());

        let pending = make_job(2, ExportFormat::Csv, "out.csv", "pending");
        assert!(!pending.is_complete());
    }

    // Test 6: job mark_done
    #[test]
    fn test_job_mark_done() {
        let mut job = make_job(3, ExportFormat::Mp4, "video.mp4", "pending");
        assert!(!job.is_complete());
        job.mark_done(5_000_000);
        assert!(job.is_complete());
        assert_eq!(job.size_bytes, Some(5_000_000));
    }

    // Test 7: queue pending + complete count
    #[test]
    fn test_queue_pending_and_complete_count() {
        let mut queue = ExportQueue::new();
        queue.enqueue(make_job(1, ExportFormat::Json, "a.json", "done"));
        queue.enqueue(make_job(2, ExportFormat::Csv, "b.csv", "pending"));
        queue.enqueue(make_job(3, ExportFormat::Pdf, "c.pdf", "pending"));

        assert_eq!(queue.complete_count(), 1);
        assert_eq!(queue.pending_count(), 2);
    }

    // Test 8: queue total_size_bytes
    #[test]
    fn test_queue_total_size_bytes() {
        let mut queue = ExportQueue::new();

        let mut j1 = make_job(1, ExportFormat::Zip, "a.zip", "pending");
        j1.mark_done(1_000);

        let mut j2 = make_job(2, ExportFormat::Wasm, "b.wasm", "pending");
        j2.mark_done(2_500);

        let j3 = make_job(3, ExportFormat::Json, "c.json", "pending");
        // j3 has no size

        queue.enqueue(j1);
        queue.enqueue(j2);
        queue.enqueue(j3);

        assert_eq!(queue.total_size_bytes(), 3_500);
    }

    // Test 9: result summary format
    #[test]
    fn test_result_summary() {
        let result = ExportResult {
            job_id: 42,
            format: ExportFormat::Mp4,
            bytes_written: 8_000_000,
        };
        assert_eq!(result.summary(), "job:42 format:mp4 bytes:8000000");
        assert!(!result.is_large());

        let large = ExportResult {
            job_id: 7,
            format: ExportFormat::Zip,
            bytes_written: 15_000_000,
        };
        assert!(large.is_large());
        assert_eq!(large.summary(), "job:7 format:zip bytes:15000000");
    }
}
