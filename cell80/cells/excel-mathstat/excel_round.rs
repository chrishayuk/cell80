//! Excel-compatible ROUND(number, num_digits): rounds number to num_digits decimal places with ties AWAY FROM ZERO (Excel's own convention, distinct from IEEE round-to-nearest-even) -- num_digits is split into a magnitude (digits_mag, 0..=10 so 10^digits_mag stays well inside f32 range) and a sign flag (digits_neg, since a state field can't be i16): pow10 = 10^digits_mag is built by a bounded repeated-multiply loop (never a variable-exponent pow), then the value is scaled UP by pow10 when digits_neg == 0 (positive num_digits, rounding to a decimal place) or scaled DOWN by pow10 when digits_neg == 1 (negative num_digits, rounding to a tens/hundreds/etc. place), rounded with a single `.round()` call -- which routes to the shipped `fround` kernel, documented in rustz80/src/softfloat.rs as "Rust's round-half-away, not RNE", i.e. already exactly Excel's ROUND tie convention with no extra tie-breaking logic needed -- and rescaled back by the inverse operation; distinct from round_to_multiple/snap_up (u16-only, ties-up, round-to-an-arbitrary-STEP rather than a decimal place) and from MROUND (rounds to a caller significance, not a power of ten), and the base convention ROUNDUP/ROUNDDOWN each override with an unconditional direction instead of a tie rule.
//! tags: excel, round, rounding, round-half-away, ties-away-from-zero, decimal-places, num-digits, negative-digits, power-of-ten, scale, fround, f32, float, softfloat, math-trig
//! kernel_bank: on
//! entry: ExcelRound::run
//! limits: digits_mag (the absolute value of num_digits) is bounded to 0..=10 -- escalates (halt 0xFF06, out_of_domain) above that, since 10^11 already starts crowding f32's useful integer-exact range for the numbers this cell is meant to round; escalates (halt 0xFF08/0xFF07, float_domain/float_overflow) if the scaled value, the rounded value, or the final rescaled result is NaN or non-finite -- reachable from a non-finite `number` input or from a finite number so large that scaling it by 10^digits_mag pushes it past f32's finite range.
struct ExcelRound {
    number: f32,
    digits_mag: u16,
    digits_neg: u16,
    result: f32,
}
impl ExcelRound {
    fn run(&mut self) -> u16 {
        if self.digits_mag > 10u16 {
            halt(0xFF06u16);
        }

        let mut pow10 = 1.0f32;
        let mut i = 0u16;
        while i < self.digits_mag {
            pow10 = pow10 * 10.0f32;
            i = i + 1u16;
        }

        let scaled = if self.digits_neg == 0u16 {
            self.number * pow10
        } else {
            self.number / pow10
        };
        if scaled.is_nan() {
            halt(0xFF08u16);
        }
        let scaled_fin = scaled.is_finite();
        if !scaled_fin {
            halt(0xFF07u16);
        }

        let rounded = scaled.round();
        if rounded.is_nan() {
            halt(0xFF08u16);
        }
        let rounded_fin = rounded.is_finite();
        if !rounded_fin {
            halt(0xFF07u16);
        }

        let result = if self.digits_neg == 0u16 {
            rounded / pow10
        } else {
            rounded * pow10
        };
        if result.is_nan() {
            halt(0xFF08u16);
        }
        let result_fin = result.is_finite();
        if !result_fin {
            halt(0xFF07u16);
        }

        self.result = result;
        1u16
    }
}
