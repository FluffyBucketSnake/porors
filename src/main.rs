use async_std::{
    stream::{Stream, StreamExt},
    task,
};
use backtrace::Backtrace;
use clap::{command, Arg};
use crossterm::{
    cursor::RestorePosition,
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, Clear, ClearType},
};
use notify_rust::Notification;
use std::{io::stdout, panic, pin::Pin, str::FromStr, time::Duration};

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
        let args = command!()
            .arg(
                Arg::new("tick-interval")
                    .short('t')
                    .required(false)
                    .value_parser(humantime::parse_duration),
            )
            .arg(
                Arg::new("work-duration")
                    .short('w')
                    .required(false)
                    .value_parser(humantime::parse_duration),
            )
            .arg(
                Arg::new("break-duration")
                    .short('b')
                    .required(false)
                    .value_parser(humantime::parse_duration),
            )
            .arg(
                Arg::new("long-break-duration")
                    .short('l')
                    .required(false)
                    .value_parser(humantime::parse_duration),
            )
            .arg(Arg::new("work-notification-icon").required(false))
            .arg(Arg::new("work-notification-title").required(false))
            .arg(Arg::new("work-notification-body").required(false))
            .arg(Arg::new("break-notification-icon").required(false))
            .arg(Arg::new("break-notification-title").required(false))
            .arg(Arg::new("break-notification-body").required(false))
            .arg(Arg::new("long-break-notification-icon").required(false))
            .arg(Arg::new("long-break-notification-title").required(false))
            .arg(Arg::new("long-break-notification-body").required(false))
            .arg(Arg::new("work-label").required(false))
            .arg(Arg::new("break-label").required(false))
            .arg(Arg::new("long-break-label").required(false))
            .get_matches();

        Self {
            tick_interval: args
                .try_get_one::<Duration>("tick-interval")
                .unwrap()
                .map_or(Duration::from_secs(1), Duration::to_owned),
            durations: PomodoroDurations {
                work_session: args
                    .try_get_one::<Duration>("work-duration")
                    .unwrap()
                    .map_or(Duration::from_secs(25 * 60), Duration::to_owned),
                break_session: args
                    .try_get_one::<Duration>("break-duration")
                    .unwrap()
                    .map_or(Duration::from_secs(5 * 60), Duration::to_owned),
                long_break_session: args
                    .try_get_one::<Duration>("long-break-duration")
                    .unwrap()
                    .map_or(Duration::from_secs(10 * 60), Duration::to_owned),
            },
            notifier: PomodoroNotifier {
                work_session_notification: (
                    *args
                        .try_get_one::<&str>("work-notification-icon")
                        .unwrap()
                        .unwrap_or(&"clock"),
                    *args
                        .try_get_one::<&str>("work-notification-title")
                        .unwrap()
                        .unwrap_or(&"Working time"),
                    *args
                        .try_get_one::<&str>("work-notification-body")
                        .unwrap()
                        .unwrap_or(&"Well, the moment has passed, back to work!"),
                )
                    .into(),
                break_session_notification: (
                    *args
                        .try_get_one::<&str>("break-notification-icon")
                        .unwrap()
                        .unwrap_or(&"clock"),
                    *args
                        .try_get_one::<&str>("break-notification-title")
                        .unwrap()
                        .unwrap_or(&"Break time"),
                    *args
                        .try_get_one::<&str>("break-notification-body")
                        .unwrap()
                        .unwrap_or(&"Drink some water!"),
                )
                    .into(),
                long_break_session_notification: (
                    *args
                        .try_get_one::<&str>("long-break-notification-icon")
                        .unwrap()
                        .unwrap_or(&"clock"),
                    *args
                        .try_get_one::<&str>("long-break-notification-title")
                        .unwrap()
                        .unwrap_or(&"Long break time"),
                    *args
                        .try_get_one::<&str>("long-break-notification-body")
                        .unwrap()
                        .unwrap_or(&"Go for a walk or eat a snack!"),
                )
                    .into(),
            },
            formatter: PomodoroDisplayFormatter {
                work_session_label: (*args
                    .try_get_one::<&str>("work-label")
                    .unwrap()
                    .unwrap_or(&"Work"))
                .into(),
                break_session_label: (*args
                    .try_get_one::<&str>("break-label")
                    .unwrap()
                    .unwrap_or(&"Break"))
                .into(),
                long_break_session_label: (*args
                    .try_get_one::<&str>("long-break-label")
                    .unwrap()
                    .unwrap_or(&"Long break"))
                .into(),
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

impl From<(&str, &str, &str)> for PomodoroNotificationTemplate {
    fn from((icon, title, body): (&str, &str, &str)) -> Self {
        Self {
            icon: icon.into(),
            title: title.into(),
            body: body.into(),
        }
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
