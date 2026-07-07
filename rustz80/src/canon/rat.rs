//! `canon::rat` — exact rational constants. Decimal literals fold to reduced `i128`
//! fractions (`"16.50"` → 33/2); no float arithmetic anywhere in the canonicalizer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rat {
    pub(crate) n: i128,
    pub(crate) d: i128, // > 0, reduced
}

fn gcd128(a: i128, b: i128) -> i128 {
    let (mut x, mut y) = (a.abs(), b.abs());
    while y != 0 {
        let t = x % y;
        x = y;
        y = t;
    }
    x
}

impl Rat {
    pub(crate) fn int(n: i128) -> Rat {
        Rat { n, d: 1 }
    }
    pub(crate) fn new(n: i128, d: i128) -> Rat {
        debug_assert!(d != 0);
        let s = if d < 0 { -1 } else { 1 };
        let g = gcd128(n, d).max(1);
        Rat {
            n: s * n / g,
            d: s * d / g,
        }
    }
    pub(crate) fn mul(self, o: Rat) -> Rat {
        Rat::new(self.n * o.n, self.d * o.d)
    }
    pub(crate) fn div(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d, self.d * o.n)
    }
    pub(crate) fn add(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
    pub(crate) fn sub(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d - o.n * self.d, self.d * o.d)
    }
    pub(crate) fn is_int(self) -> bool {
        self.d == 1
    }
    pub(crate) fn is_zero(self) -> bool {
        self.n == 0
    }
    pub(crate) fn is_one(self) -> bool {
        self.n == 1 && self.d == 1
    }
}

/// Exact decimal-literal parse: `"16.50"` → 33/2. No float arithmetic anywhere.
pub(crate) fn parse_decimal(digits: &str) -> Option<Rat> {
    if digits.contains(['e', 'E']) {
        return None; // exponent floats stay out of the dialect
    }
    let clean: String = digits.chars().filter(|c| *c != '_').collect();
    match clean.split_once('.') {
        Some((int, frac)) => {
            let scale = 10i128.checked_pow(frac.len() as u32)?;
            let int: i128 = if int.is_empty() { 0 } else { int.parse().ok()? };
            let frac: i128 = if frac.is_empty() {
                0
            } else {
                frac.parse().ok()?
            };
            Some(Rat::new(int * scale + frac, scale))
        }
        None => clean.parse().ok().map(Rat::int),
    }
}
