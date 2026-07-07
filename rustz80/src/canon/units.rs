//! `canon::units` — the unit base-scale table (fixed and versioned here, not in any
//! prompt). Money → cents, time → seconds, distance → meters, unknown nouns → count,
//! rates → explicit `numerator_per_denominator`. Versioned by
//! [`super::UNIT_TABLE_VERSION`].

/// Canonical base + multiplicative factor for one unit word. Unknown nouns are the
/// `count` convention by design (sheep, cups, GB — a count of that noun).
fn base_scale(word: &str) -> (&'static str, u32) {
    match word {
        "cents" | "cent" | "money" => ("cents", 1),
        "dollars" | "dollar" | "usd" | "bucks" | "pounds" | "gbp" | "euros" | "euro" | "eur" => {
            ("cents", 100)
        }
        "seconds" | "second" | "secs" | "sec" | "time" => ("seconds", 1),
        "minutes" | "minute" | "mins" | "min" => ("seconds", 60),
        "hours" | "hour" | "hrs" | "hr" => ("seconds", 3600),
        "days" | "day" => ("seconds", 86400),
        "weeks" | "week" => ("seconds", 604800),
        "meters" | "meter" | "metres" | "metre" | "m" | "distance" => ("meters", 1),
        "km" | "kilometers" | "kilometres" => ("meters", 1000),
        "miles" | "mile" => ("meters", 1609),
        "" | "scalar" | "ratio" => ("scalar", 1),
        "count" | "items" | "item" => ("count", 1),
        _ => ("count", 1),
    }
}

/// Normalize a unit through the base-scale table: `(canonical, num_factor, den_factor)`.
/// A rate `x_per_y` normalizes each side (`dollars_per_egg` → `("cents_per_count", 100, 1)`);
/// a plain unit has `den_factor == 1`.
pub fn canonical_unit(unit: &str) -> (String, u32, u32) {
    match unit.split_once("_per_") {
        Some((num, den)) => {
            let (nb, nf) = base_scale(num);
            let (db, df) = base_scale(den);
            (format!("{nb}_per_{db}"), nf, df)
        }
        None => {
            let (b, f) = base_scale(unit);
            (b.to_string(), f, 1)
        }
    }
}
