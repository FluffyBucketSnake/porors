use async_std::task;
use backtrace::Backtrace;
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
    io::stdout,
    panic,
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
        self.init();
        let mut previous_timestamp = Instant::now();
        loop {
            self.update_display();
            let event = self.fetch_event().await;
            let elapsed_time = previous_timestamp.elapsed();
            previous_timestamp = Instant::now();
            match event {
                PomodoroEvent::Quit => break,
                PomodoroEvent::TogglePause => self.toggle_pause(),
                PomodoroEvent::Tick if !self.paused => {
                    self.tick(elapsed_time);
                }
                _ => {}
            }
        }
        self.shutdown();
    }

    fn init(&mut self) {
        terminal::enable_raw_mode().unwrap();
        panic::set_hook(Box::new(|info| {
            execute!(stdout(), RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
            terminal::disable_raw_mode().unwrap();
            let backtrace = Backtrace::new();
            println!("{}\n{:?}", info, backtrace);
        }));
    }

    fn update_display(&self) {
        let display_text = self
            .config
            .render_display_text(&self.current_session, self.paused);
        execute!(
            stdout(),
            RestorePosition,
            Clear(ClearType::FromCursorDown),
            Print(display_text)
        )
        .unwrap();
    }

    async fn fetch_event(&mut self) -> PomodoroEvent {
        let timer = if self.paused {
            async_std::future::pending().boxed()
        } else {
            task::sleep(Duration::from_millis(1000)).boxed()
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

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    fn tick(&mut self, delta: Duration) {
        self.current_session.tick(delta);
        if self.current_session.is_finished() {
            self.show_session_end_notification();
            self.current_session = self.next_session();
        }
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

    fn shutdown(&mut self) {
        execute!(stdout(), RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

struct PomodoroConfig {
    work_session_label: String,
    break_session_label: String,
    long_break_session_label: String,
    work_session_duration: Duration,
    break_session_duration: Duration,
    long_break_session_duration: Duration,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_session_label: "Work".into(),
            break_session_label: "Break".into(),
            long_break_session_label: "Long break".into(),
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

    fn render_display_text(&self, current_session: &PomodoroSession, is_paused: bool) -> String {
        let session_kind = self.session_label_for(current_session.kind);
        let session_number = current_session.index;
        let timer = self.format_timer(current_session.remaining_time());
        if is_paused {
            format!("{session_kind}\n\rSession {session_number}\n\r{timer}\n\r(Paused)")
        } else {
            format!("{session_kind}\n\rSession {session_number}\n\r{timer}")
        }
    }

    fn session_label_for(&self, session_kind: SessionKind) -> &str {
        match session_kind {
            SessionKind::Work => &self.work_session_label,
            SessionKind::Break => &self.break_session_label,
            SessionKind::LongBreak => &self.long_break_session_label,
        }
    }

    fn format_timer(&self, timer_duration: Duration) -> String {
        let hours = timer_duration.as_secs() / 3600;
        let minutes = (timer_duration.as_secs() / 60) % 60;
        let seconds = timer_duration.as_secs() % 60;
        let milli = timer_duration.as_millis() % 1000;
        format!("{hours:02}:{minutes:02}:{seconds:02}.{milli:03}")
    }
}

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
