use std::{fmt::Display, thread, time::Duration};

fn main() {
    let working_session_duration = Duration::from_secs(25 * 60);
    let break_session_duration = Duration::from_secs(5 * 60);
    let mut elapsed_time = Duration::from_secs(0);

    loop {
        while elapsed_time < working_session_duration {
            let delta_time = working_session_duration - elapsed_time;
            println!("{}", DisplayableDuration(delta_time));
            thread::sleep(Duration::from_secs(1));
        }
    }
}

#[derive(Clone, Copy)]
struct DisplayableDuration(Duration);

impl Display for DisplayableDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DisplayableDuration(duration) = *self;
        let minutes = duration.as_secs() / 60;
        let seconds = duration.as_secs() % 60;
        write!(f, "{}:{:02}", minutes, seconds)
    }
}
