fn run() -> u16 {
    let fourth_friend_rings = 60;
    let third_friend_rings = fourth_friend_rings + 10;
    let second_friend_rings = fourth_friend_rings * 5 / 4; // 1/4 more than first friend, so 5 times the first friend's rings
    let first_friend_rings = 20;
    let total_rings = first_friend_rings + second_friend_rings + third_friend_rings + fourth_friend_rings;
    total_rings
}