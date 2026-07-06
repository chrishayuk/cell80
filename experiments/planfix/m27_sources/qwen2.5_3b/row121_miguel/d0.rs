fn run() -> u16 {
180
if weeks_in_month == 4 {
    if days_in_week == 7 {
        let total_sheets = (2 * 30) * 4;
        total_sheets
    } else {
        "Invalid days in week".to_string()
    }
} else {
    "Invalid weeks in month".to_string()
};
}