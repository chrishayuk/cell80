//! Running (stream) accumulation of the four raw sums (n, sum_x, sum_y, sum_xy) that statistics/covariance and sample_covariance_from_sums consume, one (x,y) pair per call — the missing bivariate-stream counterpart of running_variance_step's own univariate (count, sum, m2) accumulation, checked/escalating on overflow the same way rather than saturating like accumulate_step.
//! tags: running, covariance, bivariate, stream, accumulate, stats, state, wide, u32, checked, escalate
//! entry: RunningCovariance::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if sum_x, sum_y, or sum_xy would overflow u32, or if the x*y product itself overflows u32
struct RunningCovariance { x: u16, y: u16, count: u32, sum_x: u32, sum_y: u32, sum_xy: u32 }
impl RunningCovariance {
    fn run(&mut self) -> u16 {
        let xy = mul_checked_u32(self.x as u32, self.y as u32);
        self.sum_x = add_checked_u32(self.sum_x, self.x as u32);
        self.sum_y = add_checked_u32(self.sum_y, self.y as u32);
        self.sum_xy = add_checked_u32(self.sum_xy, xy);
        self.count = self.count + 1u32;
        1u16
    }
}
