use async_std::{
    stream::{Stream, StreamExt},
    task,
};
use backtrace::Backtrace;
use crossterm::{
    cursor::RestorePosition,
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, Clear, ClearType},
};
use notify_rust::Notification;
use std::{io::stdout, panic, pin::Pin, time::Duration};

fn main() {
    let config = PomodoroConfig::default();
    let app = PomodoroApplication::new(config);
    task::block_on(app.run());
}

struct PomodoroApplication {
    config: PomodoroConfig,
    paused: bool,
    event_stream: PomodoroEventStream,
    current_session: PomodoroSession,
}

impl PomodoroApplication {
    fn new(config: PomodoroConfig) -> Self {
        let initial_session = PomodoroSession::for_index(1, &config);
        let tick_interval = config.tick_interval;
        Self {
            config,
            paused: false,
            current_session: initial_session,
            event_stream: PomodoroEventStream::new(tick_interval),
        }
    }

    async fn run(mut self) {
        self.init();
        while let Some(event) = self.event_stream.next().await {
            match event {
                PomodoroEvent::Quit => break,
                PomodoroEvent::TogglePause => self.toggle_pause(),
                PomodoroEvent::Tick => self.tick(),
            }
            self.update_display();
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
        self.update_display();
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

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    fn tick(&mut self) {
        if self.paused {
            return;
        }
        self.current_session.tick(self.config.tick_interval);
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
    tick_interval: Duration,
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
            tick_interval: Duration::from_secs(1),
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
            format!("{session_kind}\n\rSession {session_number}\n\r{timer}\n\r(Paused)\n\r")
        } else {
            format!("{session_kind}\n\rSession {session_number}\n\r{timer}\n\r")
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

enum PomodoroEvent {
    Tick,
    TogglePause,
    Quit,
}

struct PomodoroEventStream {
    underlying_stream: Pin<Box<dyn Stream<Item = PomodoroEvent>>>,
}

impl PomodoroEventStream {
    fn new(tick_interval: Duration) -> Self {
        let interval_stream =
            async_std::stream::interval(tick_interval).map(|_| PomodoroEvent::Tick);
        let terminal_event = EventStream::new().filter_map(|event| match event {
            Ok(event) if event == Event::Key(KeyCode::Char('p').into()) => {
                Some(PomodoroEvent::TogglePause)
            }
            Ok(event)
                if event
                    == Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
                    || event == Event::Key(KeyCode::Char('q').into()) =>
            {
                Some(PomodoroEvent::Quit)
            }
            Ok(_) => None,
            Err(e) => panic!("{}", e),
        });
        Self {
            underlying_stream: Box::pin(interval_stream.merge(terminal_event)),
        }
    }
}

impl Stream for PomodoroEventStream {
    type Item = PomodoroEvent;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let underlying_stream = self.underlying_stream.as_mut();
        underlying_stream.poll_next(cx)
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
