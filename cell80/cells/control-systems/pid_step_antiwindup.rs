//! One discrete PID step with output clamped to [out_min, out_max] and conditional-integration anti-windup: the integral accumulator only advances when the trial (pre-clamp) output would land inside the clamp band; the instant that trial output would saturate, the accumulator freezes at last step's committed value instead of piling up further error while the actuator sits pinned at a rail -- pid_step (the plain, unclamped sibling) always accumulates every step and has no output bound at all, so given a persistent large error it winds the integral up arbitrarily far past what any real actuator could use, then overshoots badly unwinding it once the error reverses; this cell exists specifically to prevent that failure mode.
//! tags: control, controller, pid, proportional-integral-derivative, anti-windup, antiwindup, conditional-integration, integral-clamp, saturation, actuator-limit, setpoint, feedback, step, f32, float, softfloat
//! kernel_bank: on
//! entry: PidStepAntiwindup::run
//! limits: escalates (halt 0xFF06, out_of_domain) if dt == 0.0 (the derivative term divides by dt) or if out_min > out_max (an inverted clamp band); escalates (halt 0xFF08/0xFF07, float_domain/float_overflow) if the clamped output or the committed integral goes NaN or non-finite.
struct PidStepAntiwindup {
    setpoint: f32,
    measurement: f32,
    kp: f32,
    ki: f32,
    kd: f32,
    dt: f32,
    integral: f32,
    prev_error: f32,
    out_min: f32,
    out_max: f32,
    output: f32,
    integral_out: f32,
    prev_error_out: f32,
}
impl PidStepAntiwindup {
    fn run(&mut self) -> u16 {
        if self.dt == 0.0f32 {
            halt(0xFF06u16);
        }
        if self.out_min > self.out_max {
            halt(0xFF06u16);
        }

        let error = self.setpoint - self.measurement;
        let derivative = (error - self.prev_error) / self.dt;
        let integral_trial = self.integral + error * self.dt;
        let output_unclamped = self.kp * error + self.ki * integral_trial + self.kd * derivative;

        let saturated_hi = output_unclamped > self.out_max;
        let saturated_lo = output_unclamped < self.out_min;
        let saturated = saturated_hi || saturated_lo;

        let output_clamped = if saturated_hi {
            self.out_max
        } else if saturated_lo {
            self.out_min
        } else {
            output_unclamped
        };
        let integral_committed = if saturated { self.integral } else { integral_trial };

        if output_clamped.is_nan() || integral_committed.is_nan() {
            halt(0xFF08u16);
        }
        let fin1 = output_clamped.is_finite();
        let fin2 = integral_committed.is_finite();
        if !fin1 || !fin2 {
            halt(0xFF07u16);
        }

        self.output = output_clamped;
        self.integral_out = integral_committed;
        self.prev_error_out = error;
        1u16
    }
}
