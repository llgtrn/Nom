/// Bipartite min-cost assignment solver (§5.15).
///
/// Supports joint multi-app × multi-platform optimization: given a cost
/// matrix where rows = workers (nomtu × platform pairs) and columns = jobs
/// (specialization slots), the solver picks one job per worker using a greedy
/// min-cost pass. Cross-app sharing is automatic via content-addressing —
/// identical (nomtu, platform) rows alias to the same worker index.

/// Cost matrix: `costs[i][j]` = cost of assigning worker `i` to job `j`.
#[derive(Debug, Clone)]
pub struct CostMatrix {
    pub costs: Vec<Vec<f64>>,
    pub n_rows: usize,
    pub n_cols: usize,
}

impl CostMatrix {
    /// Create an `n_rows × n_cols` matrix filled with `default_cost`.
    pub fn new(n_rows: usize, n_cols: usize, default_cost: f64) -> Self {
        let costs = vec![vec![default_cost; n_cols]; n_rows];
        Self { costs, n_rows, n_cols }
    }

    /// Overwrite a single cell.
    pub fn set(&mut self, row: usize, col: usize, cost: f64) {
        self.costs[row][col] = cost;
    }

    /// Read a single cell.
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.costs[row][col]
    }

    /// Minimum cost value in a given row.
    pub fn row_min(&self, row: usize) -> f64 {
        self.costs[row]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
    }

    /// Number of columns (jobs).
    pub fn col_count(&self) -> usize {
        self.n_cols
    }
}

/// One complete assignment produced by the solver.
#[derive(Debug, Clone, Default)]
pub struct BipartiteAssignment {
    pub assignments: Vec<(usize, usize)>,
    pub total_cost: f64,
}

impl BipartiteAssignment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `worker` was assigned to `job` at `cost`.
    pub fn add(&mut self, worker: usize, job: usize, cost: f64) {
        self.assignments.push((worker, job));
        self.total_cost += cost;
    }

    /// Number of (worker, job) pairs recorded.
    pub fn len(&self) -> usize {
        self.assignments.len()
    }

    /// Returns the job index assigned to `worker`, or `None` if not assigned.
    pub fn find_job(&self, worker: usize) -> Option<usize> {
        self.assignments
            .iter()
            .find(|&&(w, _)| w == worker)
            .map(|&(_, j)| j)
    }
}

/// Greedy min-cost bipartite assignment solver.
///
/// For each row (worker) in order 0..n_rows, picks the unassigned column
/// (job) with the minimum cost. If all columns are already assigned, the
/// worker is skipped (partial assignment is acceptable for over-subscribed
/// matrices).
pub struct MinCostSolver;

impl MinCostSolver {
    pub fn new() -> Self {
        MinCostSolver
    }

    pub fn solve(&self, matrix: &CostMatrix) -> BipartiteAssignment {
        let mut result = BipartiteAssignment::new();
        let mut assigned_cols: Vec<bool> = vec![false; matrix.n_cols];

        for row in 0..matrix.n_rows {
            let mut best_col: Option<usize> = None;
            let mut best_cost = f64::INFINITY;

            for col in 0..matrix.n_cols {
                if !assigned_cols[col] {
                    let c = matrix.get(row, col);
                    if c < best_cost {
                        best_cost = c;
                        best_col = Some(col);
                    }
                }
            }

            if let Some(col) = best_col {
                assigned_cols[col] = true;
                result.add(row, col, best_cost);
            }
        }

        result
    }
}

impl Default for MinCostSolver {
    fn default() -> Self {
        MinCostSolver::new()
    }
}

#[cfg(test)]
mod bipartite_tests {
    use super::*;

    #[test]
    fn cost_matrix_new_default() {
        let m = CostMatrix::new(3, 4, 1.0);
        assert_eq!(m.n_rows, 3);
        assert_eq!(m.n_cols, 4);
        // every cell should be the default cost
        for r in 0..3 {
            for c in 0..4 {
                assert_eq!(m.get(r, c), 1.0);
            }
        }
    }

    #[test]
    fn cost_matrix_set_and_get() {
        let mut m = CostMatrix::new(2, 2, 0.0);
        m.set(0, 1, 5.5);
        assert_eq!(m.get(0, 0), 0.0);
        assert_eq!(m.get(0, 1), 5.5);
        assert_eq!(m.get(1, 0), 0.0);
    }

    #[test]
    fn cost_matrix_row_min() {
        let mut m = CostMatrix::new(2, 3, 10.0);
        m.set(0, 0, 3.0);
        m.set(0, 1, 7.0);
        m.set(0, 2, 1.5);
        assert_eq!(m.row_min(0), 1.5);
        assert_eq!(m.row_min(1), 10.0);
    }

    #[test]
    fn min_cost_solver_solve_1x1() {
        let mut m = CostMatrix::new(1, 1, 0.0);
        m.set(0, 0, 42.0);
        let sol = MinCostSolver::new().solve(&m);
        assert_eq!(sol.len(), 1);
        assert_eq!(sol.find_job(0), Some(0));
        assert!((sol.total_cost - 42.0).abs() < 1e-9);
    }

    #[test]
    fn min_cost_solver_solve_2x2_picks_min() {
        // row 0: col0=10, col1=1  → picks col1
        // row 1: col0=2,  col1=8  → col1 taken, picks col0
        let mut m = CostMatrix::new(2, 2, 0.0);
        m.set(0, 0, 10.0);
        m.set(0, 1, 1.0);
        m.set(1, 0, 2.0);
        m.set(1, 1, 8.0);
        let sol = MinCostSolver::new().solve(&m);
        assert_eq!(sol.len(), 2);
        assert_eq!(sol.find_job(0), Some(1));
        assert_eq!(sol.find_job(1), Some(0));
        assert!((sol.total_cost - 3.0).abs() < 1e-9);
    }

    #[test]
    fn min_cost_solver_solve_3x3() {
        // Greedy row-by-row on a 3×3:
        // row 0: [9, 2, 6]  → picks col1 (cost 2)
        // row 1: [3, 7, 1]  → col1 taken; picks col2 (cost 1)
        // row 2: [4, 5, 8]  → col1,col2 taken; picks col0 (cost 4)
        let mut m = CostMatrix::new(3, 3, 0.0);
        m.set(0, 0, 9.0); m.set(0, 1, 2.0); m.set(0, 2, 6.0);
        m.set(1, 0, 3.0); m.set(1, 1, 7.0); m.set(1, 2, 1.0);
        m.set(2, 0, 4.0); m.set(2, 1, 5.0); m.set(2, 2, 8.0);
        let sol = MinCostSolver::new().solve(&m);
        assert_eq!(sol.len(), 3);
        assert_eq!(sol.find_job(0), Some(1));
        assert_eq!(sol.find_job(1), Some(2));
        assert_eq!(sol.find_job(2), Some(0));
        assert!((sol.total_cost - 7.0).abs() < 1e-9);
    }

    #[test]
    fn bipartite_assignment_add_and_len() {
        let mut a = BipartiteAssignment::new();
        assert_eq!(a.len(), 0);
        a.add(0, 3, 1.0);
        a.add(1, 0, 2.5);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn bipartite_assignment_find_job() {
        let mut a = BipartiteAssignment::new();
        a.add(2, 5, 0.0);
        assert_eq!(a.find_job(2), Some(5));
        assert_eq!(a.find_job(0), None);
    }

    #[test]
    fn bipartite_assignment_total_cost_accumulates() {
        let mut a = BipartiteAssignment::new();
        a.add(0, 0, 1.25);
        a.add(1, 1, 3.75);
        a.add(2, 2, 0.50);
        assert!((a.total_cost - 5.5).abs() < 1e-9);
    }
}
