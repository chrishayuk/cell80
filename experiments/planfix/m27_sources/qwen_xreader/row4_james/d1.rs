fn run() -> u16 {
    let sprints_per_session = 3;
    let sessions_per_week = 3;
    let meters_per_sprint = 50; // Note: This is the corrected value based on the problem statement.
    
    let total_meters_per_session = sprints_per_session * meters_per_sprint;
    let total_meters_per_week = sessions_per_week * total_meters_per_session;
    
    total_meters_per_week
}