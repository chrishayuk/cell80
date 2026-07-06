fn run() -> u16 {
    let years_vacationed = (34 - 23) * 4;
    let total_blocks = years_vacationed * 1; // each year has one shirt per vacation
    total_blocks.max(0)
}