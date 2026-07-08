fn run() -> u16 {
    let sprints_per_session = 3;
    let distance_per_sprint = 60;
    let sessions_per_week = 1;

    (sprints_per_session * distance_per_sprint) * sessions_per_week
}