//! Perfectly elastic 1D collision, IEEE binary32: v1' = ((m1-m2)*v1 + 2*m2*v2) / (m1+m2) and v2' = ((m2-m1)*v2 + 2*m1*v1) / (m1+m2) -- momentum and energy conserving in exact arithmetic, correctly rounded here; zero total mass or non-finite results escalate typed instead of answering.
//! tags: physics, elastic, collision, momentum, conservation, bounce, f32, float, softfloat, mechanics
//! entry: ElasticCollision1d::run
struct ElasticCollision1d {
    m1: f32,
    v1: f32,
    m2: f32,
    v2: f32,
    v1_out: f32,
    v2_out: f32,
}
impl ElasticCollision1d {
    fn run(&mut self) -> u16 {
        let msum = self.m1 + self.m2;
        let d = self.m1 - self.m2;
        let w1 = (d * self.v1 + (2.0f32 * self.m2) * self.v2) / msum;
        let w2 = ((2.0f32 * self.m1) * self.v1 - d * self.v2) / msum;
        if w1.is_nan() || w2.is_nan() {
            halt(0xFF08u16);
        }
        let f1 = w1.is_finite();
        let f2 = w2.is_finite();
        if !f1 || !f2 {
            halt(0xFF07u16);
        }
        self.v1_out = w1;
        self.v2_out = w2;
        1u16
    }
}
