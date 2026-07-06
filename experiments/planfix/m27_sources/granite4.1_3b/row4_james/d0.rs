fn run() -> u16 {
    let sprints_per_session = 3;
    let sessions_per_week = 3;
    let meters_per_sprint = 60;

    (sprints_per_session * meters_per_sprint) * sessions_per_week
}