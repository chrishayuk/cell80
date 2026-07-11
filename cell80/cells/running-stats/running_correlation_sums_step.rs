//! Running (stream) accumulation of the six raw sums (n, sum_x, sum_y, sum_xy, sum_x2, sum_y2) that statistics/correlation and linear_regression_slope/intercept consume, one (x,y) pair per call — running_covariance_step only streams the first four (n, sum_x, sum_y, sum_xy); this widens that same accumulation with sum_x2/sum_y2 so nothing downstream needs a raw-dataset re-pass, checked/escalating on overflow the same way rather than saturating like accumulate_step.
//! tags: running, correlation, regression, bivariate, stream, accumulate, stats, state, wide, u32, checked, escalate
//! entry: RunningCorrelationSums::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if sum_x, sum_y, sum_xy, sum_x2, or sum_y2 would overflow u32, or if the x*y, x*x, or y*y product itself overflows u32
struct RunningCorrelationSums { x: u16, y: u16, count: u32, sum_x: u32, sum_y: u32, sum_xy: u32, sum_x2: u32, sum_y2: u32 }
impl RunningCorrelationSums {
    fn run(&mut self) -> u16 {
        let xy = mul_checked_u32(self.x as u32, self.y as u32);
        let x2 = mul_checked_u32(self.x as u32, self.x as u32);
        let y2 = mul_checked_u32(self.y as u32, self.y as u32);
        self.sum_x = add_checked_u32(self.sum_x, self.x as u32);
        self.sum_y = add_checked_u32(self.sum_y, self.y as u32);
        self.sum_xy = add_checked_u32(self.sum_xy, xy);
        self.sum_x2 = add_checked_u32(self.sum_x2, x2);
        self.sum_y2 = add_checked_u32(self.sum_y2, y2);
        self.count = self.count + 1u32;
        1u16
    }
}
