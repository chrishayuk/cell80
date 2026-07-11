//! Excel-compatible DEGREES(angle): converts an angle from radians to degrees, degrees = angle * (180/pi) -- computed as a single f32 constant multiply (angle times a precomputed 57.29577951308232 literal, 180/pi rounded to the nearest f32) through the fmul kernel alone, no division, no iteration, and no trig call, with no domain check beyond the shared NaN/non-finite output guard every f32 cell in this pack already runs; the simplest cell in the whole Excel Math&Trig/Statistical batch, distinct from RADIANS (the reverse angle-to-radians conversion, angle * pi/180, a still-unbuilt Q8.8 fixed-point candidate per docs/excel-mathstat-map.md rather than an f32 cell) by both direction and representation, and distinct from excel_sqrt/excel_odd/excel_round (this pack's other f32 cells) by needing no follow-up operation at all after the multiply -- no sqrt, no ceil/floor, no rounding, no sign handling.
//! tags: excel, degrees, radians-to-degrees, angle-conversion, radians, constant-multiply, fmul, f32, float, softfloat, math-trig
//! entry: ExcelDegrees::run
//! limits: no domain restriction -- every finite f32 angle has a defined result (unlike excel_sqrt's number>=0 requirement or excel_odd/excel_round's magnitude bounds, DEGREES never escalates on the input value itself); escalates (halt 0xFF08, float_domain) only if the computed result is NaN, (halt 0xFF07, float_overflow) only if it is non-finite -- both reachable only when angle itself was already NaN/non-finite, since a finite angle times a finite constant is always finite.
struct ExcelDegrees {
    angle: f32,
    result: f32,
}
impl ExcelDegrees {
    fn run(&mut self) -> u16 {
        let r = self.angle * 57.29577951308232f32;
        if r.is_nan() {
            halt(0xFF08u16);
        }
        let r_fin = r.is_finite();
        if !r_fin {
            halt(0xFF07u16);
        }
        self.result = r;
        1u16
    }
}
