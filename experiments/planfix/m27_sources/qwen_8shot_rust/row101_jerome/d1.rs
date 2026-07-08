fn run() -> u16 {
    let fourth_friend = 60;
    let third_friend = fourth_friend + 10;
    let second_friend = fourth_friend * 5 / 4; // 1/4 more than first friend
    let first_friend = second_friend;
    first_friend + second_friend + third_friend + fourth_friend
}