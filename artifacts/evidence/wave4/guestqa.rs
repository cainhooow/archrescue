use std::{env, time::Duration, thread};
#[link(name="user32")]
extern "system" {
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn mouse_event(flags: u32, x: u32, y: u32, data: u32, info: usize);
}
fn main() {
    thread::sleep(Duration::from_secs(3));
    let coordinates: Vec<i32> = env::args().skip(1).map(|value| value.parse().unwrap()).collect();
    for pair in coordinates.chunks_exact(2) {
        unsafe {
            SetCursorPos(pair[0],pair[1]);
            thread::sleep(Duration::from_millis(80));
            mouse_event(2,0,0,0,0);
            thread::sleep(Duration::from_millis(70));
            mouse_event(4,0,0,0,0);
        }
        thread::sleep(Duration::from_millis(1300));
    }
}
