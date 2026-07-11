//! Bang-bang (on/off) controller: returns +1 (drive up) if value is below setpoint-deadband, -1 (drive down) if value is above setpoint+deadband, or 0 (hold) inside the deadband window -- unlike sign_i16 (a raw three-way sign of one already-signed value, no window at all) or hysteresis (a *stateful* Schmitt-trigger latch that must remember its own prior on/off state to decide the dead zone), this is a stateless three-way decision computed fresh from value/setpoint/deadband alone: the deadband window is what stops the rapid on/off chattering a plain value<setpoint comparison would cause exactly at the setpoint.
//! tags: control, controller, bang-bang, on-off, on/off, relay, thermostat, setpoint, deadband, dead-band, dead-zone, drive, actuator, three-way, stateless, threshold
fn run(value: u16, setpoint: u16, deadband: u16) -> i16 {
    // Drive-up test: value < setpoint - deadband, rearranged to value + deadband < setpoint
    // so no subtraction (and no underflow when deadband > setpoint) is ever needed.
    let sum_lo = value.wrapping_add(deadband);
    let lo_overflowed = sum_lo < value;

    // Drive-down test: value > setpoint + deadband, guarded the same way against
    // setpoint + deadband wrapping past u16::MAX.
    let sum_hi = setpoint.wrapping_add(deadband);
    let hi_overflowed = sum_hi < setpoint;

    if !lo_overflowed && sum_lo < setpoint {
        1i16
    } else if !hi_overflowed && value > sum_hi {
        -1i16
    } else {
        0i16
    }
}
