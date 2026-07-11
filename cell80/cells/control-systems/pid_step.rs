//! One PID (proportional-integral-derivative) controller step: output = kp*error + ki*integral' + kd*(error-prev_error)/dt, where integral' = integral + error*dt is accumulated before use and both integral and prev_error persist as round-trip state for the next call -- cell80's first closed-loop, error-driven feedback controller, distinct from the agentic-runtime pack's open-loop signal gates (hysteresis, debounce_step, token_bucket_step), which react to a raw signal or event count rather than an accumulated/derivative error history, and from spring_damper_step_f32 (physics pack) whose acceleration comes from a fixed physical law rather than caller-supplied gains against an arbitrary setpoint error.
//! tags: control, controller, pid, proportional, integral, derivative, feedback, closed-loop, setpoint, error, gain, windup, step, control-systems, f32, float, softfloat
//! kernel_bank: on
//! entry: PidStep::run
//! limits: escalates (halt 0xFF06, out_of_domain) if dt == 0.0 (division by zero in the derivative term); escalates (halt 0xFF08/0xFF07, float_domain/float_overflow) if the updated integral or output goes NaN or non-finite; integral and prev_error are round-trip state fields the caller threads through repeated calls (initialize integral=0.0 and prev_error to the first call's error to avoid a derivative kick on step one), the same threading convention spring_damper_step_f32's x/v and token_bucket_step's tokens use.
struct PidStep {
    error: f32,
    dt: f32,
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    prev_error: f32,
    output: f32,
}
impl PidStep {
    fn run(&mut self) -> u16 {
        if self.dt == 0.0f32 {
            halt(0xFF06u16);
        }

        let new_integral = self.integral + self.error * self.dt;
        let derivative = (self.error - self.prev_error) / self.dt;
        let out = self.kp * self.error + self.ki * new_integral + self.kd * derivative;

        if out.is_nan() || new_integral.is_nan() {
            halt(0xFF08u16);
        }
        let out_fin = out.is_finite();
        let int_fin = new_integral.is_finite();
        if !out_fin || !int_fin {
            halt(0xFF07u16);
        }

        self.integral = new_integral;
        self.prev_error = self.error;
        self.output = out;
        1u16
    }
}
