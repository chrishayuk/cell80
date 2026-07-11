//! Excel-compatible RADIANS(angle): converts a whole number of degrees to radians in Q8.8 fixed-point, radians = degrees * pi / 180, computed as (degrees * 804) / 180 at wide u32 width using the same Q8.8 pi constant (804 = round(pi*256)) geom_circle_area_approx/geom_circle_circumference_approx already established -- unlike those two cells' squared-radius and doubled-radius circle formulas, this is a bare degrees-to-radians unit rescale with no existing cell to compose it from, since RADIANS needs no transcendental, only a scalar multiply-then-divide by the same fixed pi the fixed-point pack already owns.
//! tags: excel, radians, degrees, angle, conversion, unit-conversion, pi, fixed-point, q8.8, wide, u32, checked, escalate, math-trig
//! limits: degrees is a plain (non-Q8.8) whole number of degrees, not itself Q8.8-scaled; the returned value is Q8.8-scaled radians (divide by 256 for the true radian value); escalates (halt 0xFF05, needs_wider_math) if degrees*804/180 would exceed u16::MAX (degrees >= 14673)
fn run(degrees: u16) -> u16 {
    let scaled = degrees as u32 * 804u32;
    let result = scaled / 180u32;
    if result > 65535u32 {
        halt(0xFF05u16);
    }
    result as u16
}
