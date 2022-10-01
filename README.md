# ~ :tomato: pomodoro-rs :tomato: ~

A quick and simple CLI implementation of a *Pomodoro* timer.

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
- [ ] Long **break** sessions;
- [ ] Pausing the timer;
- [ ] Command line configuration for session duration, timer display and notification texts.

### Possible extra features

- [ ] File configuration alternative;
- [ ] Custom alarm sound.

## Dependencies

This project currently uses the following crates:

- *crossterm 0.25.0*
- *notify-rust 4.5.10*

If you are on Linux, you'll probably need **D-Bus** and a **notification server**. Both normally come with pretty much all *DEs*, such as *GNOME* and *KDE*. However, if you are on a minimalist distro, you might need to install both. As a recommendation for a notification server, I'll mention [Dunst](https://github.com/dunst-project/dunst).

## Building

Just use `cargo run`

## Running

As of now, there are no command line options. So:

```bash
pomodoro-rs
```
