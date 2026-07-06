fn run() -> u16 {
    let sprints_per_session = 3;
    let sessions_per_week = 3;
    let distance_per_sprint = 60;

    // Total meters = (Sprints/Session) * (Sessions/Week) * (Meters/Sprint)
    (sprints_per_session * sessions_per_week * distance_per_sprint)
}