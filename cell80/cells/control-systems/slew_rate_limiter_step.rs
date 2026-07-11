//! Limits how much a signal can change in one step: moves current toward target by at most max_delta_per_step in either direction -- the "can't change too fast" rate-limit primitive real actuators/motors need, distinct from clamp (bounds an absolute VALUE against a fixed [lo,hi] range, no notion of a previous state or a per-step rate) and hysteresis (a threshold LATCH between two fixed states, not a bounded step toward an arbitrary target).
//! tags: control, control-systems, rate-limit, slew-rate, slew, ramp, ramp-rate, actuator, motor, servo, step, rate-of-change, approach, move-toward, governor
fn run(current: u16, target: u16, max_delta_per_step: u16) -> u16 {
    if target > current {
        let diff = target - current;
        let step = if diff > max_delta_per_step { max_delta_per_step } else { diff };
        current + step
    } else if current > target {
        let diff = current - target;
        let step = if diff > max_delta_per_step { max_delta_per_step } else { diff };
        current - step
    } else {
        current
    }
}
