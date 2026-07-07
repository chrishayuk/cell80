//! Returns 1 if applying a signed delta to an unsigned value would stay nonnegative, else 0 — the boolean-verdict form of the sign-handling idiom apply_delta_clamped already uses, for a caller (e.g. a plan verifier) that wants to kill a wrong "subtract too much" plan cheaply without needing the clamped value itself.
//! tags: verify, verifier, delta, signed, i16, nonnegative, predicate, risk, resource, check, plan
fn run(value: u16, delta: i16) -> u16 {
    if delta >= 0i16 {
        1u16
    } else {
        let mag = 0u16.wrapping_sub(delta as u16);
        (value >= mag) as u16
    }
}
