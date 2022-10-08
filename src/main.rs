use async_std::task;
use crossterm::{
    cursor::RestorePosition,
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, Clear, ClearType},
};
use futures::{pin_mut, select, FutureExt, StreamExt};
use notify_rust::Notification;
use std::{
    fmt::Display,
    io::stdout,
    time::{Duration, Instant},
};

fn main() {
    let config = PomodoroConfig::default();
    let app = PomodoroApplication::new(config);
    task::block_on(app.run());
}

enum PomodoroEvent {
    Tick,
    TogglePause,
    Quit,
}

struct PomodoroApplication {
    config: PomodoroConfig,
    paused: bool,
    current_session: PomodoroSession,
    terminal_stream_pool: EventStream,
}

impl PomodoroApplication {
    fn new(config: PomodoroConfig) -> Self {
        let initial_session = PomodoroSession::for_index(1, &config);
        Self {
            config,
            paused: false,
            current_session: initial_session,
            terminal_stream_pool: EventStream::new(),
        }
    }

    async fn run(mut self) {
        terminal::enable_raw_mode().unwrap();
        let mut previous_timestamp = Instant::now();
        loop {
            let event = self.fetch_event().await;
            let elapsed_time = previous_timestamp.elapsed();
            previous_timestamp = Instant::now();
            match event {
                PomodoroEvent::Quit => break,
                PomodoroEvent::TogglePause => {
                    if !self.paused {
                        self.tick(elapsed_time);
                    }
                    self.paused = !self.paused;
                }
                PomodoroEvent::Tick if !self.paused => {
                    self.tick(elapsed_time);
                }
                _ => {}
            }
        }
        execute!(stdout(), RestorePosition).unwrap();
        terminal::disable_raw_mode().unwrap();
    }

    async fn fetch_event(&mut self) -> PomodoroEvent {
        let timer = if self.paused {
            async_std::future::pending().boxed()
        } else {
            task::sleep(ONE_SECOND).boxed()
        }
        .fuse();
        let terminal_event = self.terminal_stream_pool.next().fuse();

        pin_mut!(timer, terminal_event);

        select!(
            () = timer => PomodoroEvent::Tick,
            event = terminal_event => {
                match event {
                    Some(Ok(event))
                        if event == Event::Key(KeyCode::Char('p').into()) => PomodoroEvent::TogglePause,
                    Some(Ok(event))
                        if event == Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
                        || event == Event::Key(KeyCode::Char('q').into()) => PomodoroEvent::Quit,
                    Some(Ok(_)) => PomodoroEvent::Tick,
                    Some(Err(e)) => panic!("{}", e),
                    None => PomodoroEvent::Quit
                }
            }
        )
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
