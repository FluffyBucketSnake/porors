use crossterm::{
    cursor::{RestorePosition, SavePosition},
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use std::{fmt::Display, io::stdout, thread, time::Duration};

fn main() {
    let config = PomodoroConfig::default();
    let mut current_session = PomodoroSession::first(&config);

    execute!(stdout(), SavePosition).unwrap();
    loop {
        while !current_session.is_finished() {
            execute!(
                stdout(),
                RestorePosition,
                Clear(ClearType::FromCursorDown),
                Print(&current_session)
            )
            .unwrap();
            let delta_time = ONE_SECOND;
            thread::sleep(delta_time);
            current_session.tick(delta_time);
        }
        current_session = current_session.next();
    }
}

struct PomodoroConfig {
    work_session_duration: Duration,
    break_session_duration: Duration,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_session_duration: Duration::from_secs(1 * 60),
            break_session_duration: Duration::from_secs(30),
        }
    }
}

impl PomodoroConfig {
    fn session_duration_for(&self, session_kind: SessionKind) -> Duration {
        match session_kind {
            SessionKind::Work => self.work_session_duration,
            SessionKind::Break => self.break_session_duration,
        }
    }
}

const ONE_SECOND: Duration = Duration::from_secs(1);

enum SessionKind {
    Work,
    Break,
}

struct PomodoroSession<'a> {
    index: usize,
    elapsed_time: Duration,
    config: &'a PomodoroConfig,
}

impl<'a> PomodoroSession<'a> {
    fn first(config: &'a PomodoroConfig) -> Self {
        Self {
            index: 1,
            elapsed_time: Duration::ZERO,
            config,
        }
    }

    fn kind(&self) -> SessionKind {
        match self.index % 2 {
            0 => SessionKind::Break,
            1 => SessionKind::Work,
            _ => unreachable!(),
        }
    }

    fn duration(&self) -> Duration {
        self.config.session_duration_for(self.kind())
    }

    fn time_till_end(&self) -> Duration {
        self.duration() - self.elapsed_time
    }

    fn is_finished(&self) -> bool {
        self.elapsed_time > self.duration()
    }

    fn tick(&mut self, delta_time: Duration) {
        self.elapsed_time += delta_time;
    }

    fn next(&self) -> Self {
        Self {
            index: self.index + 1,
            elapsed_time: Duration::ZERO,
            config: self.config,
        }
    }
}

impl<'a> Display for PomodoroSession<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind_text = match self.kind() {
            SessionKind::Work => "Work",
            SessionKind::Break => "Break",
        };
        let time_till_end = self.time_till_end();
        let timer_minutes = time_till_end.as_secs() / 60;
        let timer_seconds = time_till_end.as_secs() % 60;
        write!(
            f,
            "Session {}: ({}); Timer: {:02}:{:02}",
            self.index, kind_text, timer_minutes, timer_seconds
        )
    }
}
