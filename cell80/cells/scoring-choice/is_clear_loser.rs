//! Returns 1 if the bottom score is beaten by the second-lowest by at least margin (a decisive last place, not a near-tie), else 0 — including when bottom > second_lowest (a malformed call, treated as no clear loser) — the bottom-side counterpart of is_clear_winner/clear_winner3/clear_winner_u32, which all only check for a decisive winner at the top.
//! tags: loser, clear, margin, decisive, tie, ambiguous, score, ranking, bottom, last, plan
fn run(bottom: u16, second_lowest: u16, margin: u16) -> u16 {
    if bottom > second_lowest {
        0u16
    } else {
        ((second_lowest - bottom) >= margin) as u16
    }
}
