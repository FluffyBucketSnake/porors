use async_std::task;
use crossterm::{
    cursor::RestorePosition,
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use notify_rust::Notification;
use std::{fmt::Display, io::stdout, time::Duration};

fn main() {
    let config = PomodoroConfig::default();
    let app = PomodoroApplication::new(config);
    task::block_on(app.run());
}

struct PomodoroApplication {
    config: PomodoroConfig,
    current_session: PomodoroSession,
}

impl PomodoroApplication {
    fn new(config: PomodoroConfig) -> Self {
        let initial_session = PomodoroSession::for_index(1, &config);
        Self {
            config,
            current_session: initial_session,
        }
    }

    async fn run(mut self) {
        loop {
            let delta_time = ONE_SECOND;
            task::sleep(delta_time).await;
            self.tick(delta_time);
        }
    }

    fn tick(&mut self, delta: Duration) {
        self.display_session();
        self.current_session.tick(delta);
        if self.current_session.is_finished() {
            self.show_session_end_notification();
            self.current_session = self.next_session();
        }
    }

    fn display_session(&self) {
        execute!(
            stdout(),
            RestorePosition,
            Clear(ClearType::FromCursorDown),
            Print(&self.current_session)
        )
        .unwrap();
    }

    fn show_session_end_notification(&self) {
        Notification::new()
            .summary("Pomodoro session over")
            .icon("clock")
            .show()
            .unwrap();
    }

    fn next_session(&self) -> PomodoroSession {
        PomodoroSession::for_index(self.current_session.index + 1, &self.config)
    }
}

struct PomodoroConfig {
    work_session_duration: Duration,
    break_session_duration: Duration,
    long_break_session_duration: Duration,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_session_duration: Duration::from_secs(1 * 60),
            break_session_duration: Duration::from_secs(10),
            long_break_session_duration: Duration::from_secs(30),
        }
    }
}

impl PomodoroConfig {
    fn session_duration_for(&self, session_kind: SessionKind) -> Duration {
        match session_kind {
            SessionKind::Work => self.work_session_duration,
            SessionKind::Break => self.break_session_duration,
            SessionKind::LongBreak => self.long_break_session_duration,
        }
    }
}

const ONE_SECOND: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
enum SessionKind {
    Work,
    Break,
    LongBreak,
}

impl SessionKind {
    fn for_index(index: usize) -> Self {
        if index % 8 == 0 {
            Self::LongBreak
        } else if index % 2 == 0 {
            Self::Break
        } else {
            Self::Work
        }
    }
}

struct PomodoroSession {
    index: usize,
    kind: SessionKind,
    elapsed_time: Duration,
    duration: Duration,
}

impl PomodoroSession {
    fn for_index(index: usize, config: &PomodoroConfig) -> Self {
        let kind = SessionKind::for_index(index);
        Self {
            index,
            kind,
            duration: config.session_duration_for(kind),
            elapsed_time: Duration::ZERO,
        }
    }

    fn remaining_time(&self) -> Duration {
        self.duration - self.elapsed_time
    }

    fn is_finished(&self) -> bool {
        self.elapsed_time > self.duration
    }

    fn tick(&mut self, delta_time: Duration) {
        self.elapsed_time += delta_time;
    }
}

impl Display for PomodoroSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind_text = match self.kind {
            SessionKind::Work => "Work",
            SessionKind::Break => "Break",
            SessionKind::LongBreak => "Long break",
        };
        let time_till_end = self.remaining_time();
        let timer_minutes = time_till_end.as_secs() / 60;
        let timer_seconds = time_till_end.as_secs() % 60;
        write!(
            f,
            "Session {}: ({}); Timer: {:02}:{:02}",
            self.index, kind_text, timer_minutes, timer_seconds
        )
    }
}
