//! Returns 1 if the top score beats the second-best by at least margin (a decisive win, not a near-tie), else 0 — including when top < second (a malformed call, treated as no clear winner).
//! tags: winner, clear, margin, decisive, tie, ambiguous, score, ranking, plan
fn run(top: u16, second: u16, margin: u16) -> u16 {
    if top < second {
        0u16
    } else {
        ((top - second) >= margin) as u16
    }
}
