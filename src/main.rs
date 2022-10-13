use async_std::{
    stream::{Stream, StreamExt},
    task,
};
use backtrace::Backtrace;
use clap::Parser;
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
    let config = task::block_on(PomodoroConfig::load());
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
        self.show_session_start_notification();
        self.update_display();
    }

    fn update_display(&self) {
        let display_text = self
            .config
            .formatter
            .format_session(&self.current_session, self.paused);
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
            self.go_to_next_session();
        }
    }

    fn show_session_start_notification(&self) {
        self.config
            .notifier
            .notify_session_start(&self.current_session);
    }

    fn go_to_next_session(&mut self) {
        self.current_session =
            PomodoroSession::for_index(self.current_session.index + 1, &self.config);
        self.show_session_start_notification();
    }

    fn shutdown(&mut self) {
        execute!(stdout(), RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

struct PomodoroConfig {
    tick_interval: Duration,
    durations: PomodoroDurations,
    formatter: PomodoroDisplayFormatter,
    notifier: PomodoroNotifier,
}

impl PomodoroConfig {
    async fn load() -> Self {
        let args = PomodoroArgs::parse();

        Self {
            tick_interval: args.tick_interval.unwrap_or(Duration::from_secs(1)),
            durations: PomodoroDurations {
                work_session: args.work_duration.unwrap_or(Duration::from_secs(25 * 60)),
                break_session: args.break_duration.unwrap_or(Duration::from_secs(5 * 60)),
                long_break_session: args
                    .long_break_duration
                    .unwrap_or(Duration::from_secs(10 * 60)),
            },
            notifier: PomodoroNotifier {
                work_session_notification: (
                    args.work_notification_icon.unwrap_or("clock".into()),
                    args.work_notification_title
                        .unwrap_or("Working time".into()),
                    args.work_notification_body
                        .unwrap_or("Well, the moment has passed, back to work!".into()),
                )
                    .into(),
                break_session_notification: (
                    args.break_notification_icon.unwrap_or("clock".into()),
                    args.break_notification_title.unwrap_or("Break time".into()),
                    args.break_notification_body
                        .unwrap_or("Drink some water!".into()),
                )
                    .into(),
                long_break_session_notification: (
                    args.long_break_notification_icon.unwrap_or("clock".into()),
                    args.long_break_notification_title
                        .unwrap_or("Long break time".into()),
                    args.long_break_notification_body
                        .unwrap_or("Go for a walk or eat a snack!".into()),
                )
                    .into(),
            },
            formatter: PomodoroDisplayFormatter {
                work_session_label: args.work_label.unwrap_or("Work".into()),
                break_session_label: args.break_label.unwrap_or("Break".into()),
                long_break_session_label: args.long_break_label.unwrap_or("Long break".into()),
            },
        }
    }
}

struct PomodoroDurations {
    work_session: Duration,
    break_session: Duration,
    long_break_session: Duration,
}

impl PomodoroDurations {
    fn for_session(&self, kind: SessionKind) -> Duration {
        match kind {
            SessionKind::Work => self.work_session,
            SessionKind::Break => self.break_session,
            SessionKind::LongBreak => self.long_break_session,
        }
    }
}

struct PomodoroDisplayFormatter {
    work_session_label: String,
    break_session_label: String,
    long_break_session_label: String,
}

impl PomodoroDisplayFormatter {
    fn format_session(&self, session: &PomodoroSession, paused: bool) -> String {
        let session_kind = self.session_label_for(session.kind);
        let session_number = session.index;
        let timer = self.format_timer(session.remaining_time());
        if paused {
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

struct PomodoroNotifier {
    work_session_notification: PomodoroNotificationTemplate,
    break_session_notification: PomodoroNotificationTemplate,
    long_break_session_notification: PomodoroNotificationTemplate,
}

impl PomodoroNotifier {
    fn notify_session_start(&self, session: &PomodoroSession) {
        let notification = match session.kind {
            SessionKind::Work => &self.work_session_notification,
            SessionKind::Break => &self.break_session_notification,
            SessionKind::LongBreak => &self.long_break_session_notification,
        }
        .build();
        notification.show().unwrap();
    }
}

struct PomodoroNotificationTemplate {
    icon: String,
    title: String,
    body: String,
}

impl PomodoroNotificationTemplate {
    fn build(&self) -> Notification {
        let mut notification = Notification::new();
        notification
            .icon(&self.icon)
            .summary(&self.title)
            .body(&self.body);
        notification
    }
}

impl<T: Into<String>, U: Into<String>, V: Into<String>> From<(T, U, V)>
    for PomodoroNotificationTemplate
{
    fn from((icon, title, body): (T, U, V)) -> Self {
        Self {
            icon: icon.into(),
            title: title.into(),
            body: body.into(),
        }
    }
}

#[derive(Parser)]
struct PomodoroArgs {
    #[arg(short = 't', long, value_parser = humantime::parse_duration, value_name = "DURATION")]
    tick_interval: Option<Duration>,

    #[arg(short = 'w', long, value_parser = humantime::parse_duration, value_name = "DURATION")]
    work_duration: Option<Duration>,

    #[arg(short = 'b', long, value_parser = humantime::parse_duration, value_name = "DURATION")]
    break_duration: Option<Duration>,

    #[arg(short = 'l', long, value_parser = humantime::parse_duration, value_name = "DURATION")]
    long_break_duration: Option<Duration>,

    #[arg(long, value_name = "ICON")]
    work_notification_icon: Option<String>,

    #[arg(long, value_name = "TEXT")]
    work_notification_title: Option<String>,

    #[arg(long, value_name = "TEXT")]
    work_notification_body: Option<String>,

    #[arg(long, value_name = "ICON")]
    break_notification_icon: Option<String>,

    #[arg(long, value_name = "TEXT")]
    break_notification_title: Option<String>,

    #[arg(long, value_name = "TEXT")]
    break_notification_body: Option<String>,

    #[arg(long, value_name = "ICON")]
    long_break_notification_icon: Option<String>,

    #[arg(long, value_name = "TEXT")]
    long_break_notification_title: Option<String>,

    #[arg(long, value_name = "TEXT")]
    long_break_notification_body: Option<String>,

    #[arg(long, value_name = "TEXT")]
    work_label: Option<String>,

    #[arg(long, value_name = "TEXT")]
    break_label: Option<String>,

    #[arg(long, value_name = "TEXT")]
    long_break_label: Option<String>,
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
            duration: config.durations.for_session(kind),
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
