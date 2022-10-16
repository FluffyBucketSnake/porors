# ~ :tomato: porors :tomato: ~

`porors`. A quick and simple CLI implementation of a *Pomodoro* timer.

## What the heck is a Pomodoro?

The *Pomodoro* technique is a time management method, developed by *Francesco Cirillo*, supposed to increase productivity. It accomplishes such by splitting work into sessions - which alternate between long **work** and small **break** sessions.

Also, *pomodoro* is Italian for tomato. Just letting you know :)

## So, why does this project exist

1. I wanted a simple CLI app for tracking *Pomodoro* sessions. Just a single timer. No daemon/service. Keep it simple, stupid.

2. Practice with the *Rust* programming language, specially *asynchronous Rust*.

3. Fun. Coding is fun. 'nuff said.

## Roadmap

- [x] Track **work** and **break** sessions;
- [x] Show notifications on each session end;
- [x] Long **break** sessions;
- [x] Pausing the timer;
- [x] Command line configuration for session duration, timer display and notification texts;
- [x] Handle system signals correctly (SIGINT|SIGQUIT|SIGTERM -> Quit; SIGUSR1 -> Pause/Resume);
- [x] Use `anyhow` for better error handling.

### Possible extra features

- [ ] File configuration alternative;
- [ ] Customizing the timer display;
- [ ] Resetting the timer;
- [ ] Custom alarm sound.

## Dependencies

This project currently uses the following crates:

- async-std = "1.12.0"
- backtrace = "0.3.66"
- clap = "4.0.13"
- crossterm = "0.25.0"
- dynfmt = "0.1.5"
- humantime = "2.1.0"
- notify-rust = "4.5.10"
- serde = "1.0.145"

If you are on Linux, you'll probably need **D-Bus** and a **notification server**. Both normally come with pretty much all *DEs*, such as *GNOME* and *KDE*. However, if you are on a minimalist distro, you might need to install both. As a recommendation for a notification server, I'll mention [Dunst](https://github.com/dunst-project/dunst).

## Building

Just use `cargo build`

## Running

To run the binary, all you need is:

```bash
porors
```

It will start a Pomodoro timer tracker with the default settings.

However, if you need a more specialized timer, you can view the options with `-h` or `--help`. Here are the built-in options:

### Timer options

These options change the behavior of the timer itself.

- `-t, --tick-interval <DURATION>`: changes the time between each timer tick. Setting it into `3s`, for instance, means the clock will update each 3 seconds;
- `-w, --work-duration <DURATION>`: changes the duration of all work sessions;
- `-b, --break-duration <DURATION>`: changes the duration of all break sessions;
- `-l, --long-break-duration <DURATION>`: changes the duration of all long break sessions.

### Notification options

These options change the behavior of the session end notifications.

- `--work-notification-icon <ICON>`/`--break-notification-icon <ICON>`/`--long-break-notification-icon <ICON>`: changes notification icon;
- `--work-notification-title <TEXT>`/`--break-notification-title <TEXT>`/`--long-break-notification-title <TEXT>`: changes notification title;
- `--break-notification-body <TEXT>`/`--work-notification-body <TEXT>`/`--long-break-notification-body <TEXT>`: changes notification body text;

### Display options

These options change the behavior of the CLI display.

- `--active-display <TEXT>`: changes the format string used by the display while the timer is active(ie. not paused);
- `--paused-display <TEXT>`: changes the format string used by the display while the timer is paused;
- `--work-label <TEXT>`/`--break-label <TEXT>`/`--long-break-label <TEXT>`: changes the label used for each respective type of session.

Each format string accepts three format tokens:

- `{timer}`: the timer display itself, shows the remaining time for the current session;
- `{session_kind}`: label for the current session kind;
- `{session_number}`: number of the current session.
