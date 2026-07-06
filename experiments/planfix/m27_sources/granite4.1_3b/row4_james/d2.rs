fn run() -> u16 {
    let per_run = 3 * 60; // 3 sprints * 60 meters per sprint
    let weekly_runs = per_run * 3; // done 3 times a week
    (weekly_runs / 1000) as u16
}