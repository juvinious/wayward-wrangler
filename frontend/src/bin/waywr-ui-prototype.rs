//! Standalone scripted UI evaluation. No production IPC, Herdr, Python or model access.
use waywr::prototype::view;

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env, fs,
    io::{self, stdout},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use view::App;

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_signal: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(
            stdout(),
            EnterAlternateScreen,
            EnableBracketedPaste, EnableMouseCapture,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        ) {
            let _ = execute!(
                stdout(),
                PopKeyboardEnhancementFlags,
                DisableBracketedPaste, DisableMouseCapture,
                LeaveAlternateScreen
            );
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            stdout(),
            PopKeyboardEnhancementFlags,
            DisableBracketedPaste, DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

fn arguments() -> io::Result<PathBuf> {
    let mut args = env::args_os().skip(1);
    let mut scenario = None;
    let mut dir = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--scenario") => scenario = args.next().and_then(|v| v.into_string().ok()),
            Some("--data-dir") => dir = args.next().map(PathBuf::from),
            Some("--help") | Some("-h") => {
                println!(
                    "waywr-ui-prototype --scenario decision-log --data-dir DIR\nSCRIPTED UI demonstration only. No production IPC, services or model calls."
                );
                std::process::exit(0);
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Only --scenario decision-log and --data-dir DIR are supported",
                ));
            }
        }
    }
    if scenario.as_deref() != Some("decision-log") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Specify --scenario decision-log",
        ));
    }
    let dir = dir.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Specify an isolated --data-dir",
        )
    })?;
    if dir
        .components()
        .any(|c| c.as_os_str() == ".waywr")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Do not use a production .waywr directory for scripted data",
        ));
    }
    if dir.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Scripted data directory must not be a symlink",
        ));
    }
    fs::create_dir_all(&dir)?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}

fn run(dir: PathBuf) -> io::Result<()> {
    let mut app = App::new(dir)?;
    let _guard = TerminalGuard::enter()?;
    unsafe {
        libc::signal(
            libc::SIGTERM,
            stop_signal as *const () as libc::sighandler_t,
        );
        libc::signal(libc::SIGINT, stop_signal as *const () as libc::sighandler_t);
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut last_frame = Instant::now();
    while !app.detached && !STOP.load(Ordering::SeqCst) {
        if app.tick() {
            app.save()?;
        }
        let effect_ready =
            app.effects_running() && last_frame.elapsed() >= Duration::from_millis(34);
        if app.dirty || effect_ready {
            let elapsed = last_frame.elapsed();
            terminal.draw(|frame| app.render(frame, elapsed))?;
            app.dirty = false;
            last_frame = Instant::now();
        }
        if app.persist_pending && app.saved_at.elapsed() >= Duration::from_millis(250) {
            app.save()?;
        }
        let until_script = app
            .next_due
            .map(|due| due.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_millis(250));
        let wait = if app.effects_running() {
            Duration::from_millis(34)
        } else {
            Duration::from_millis(250)
        };
        if event::poll(wait.min(until_script))? {
            let input = event::read()?;
            app.input(input);
        }
    }
    app.save()?;
    Ok(())
}

fn main() {
    let result = arguments().and_then(run);
    if let Err(error) = result {
        eprintln!("waywr-ui-prototype: {error}");
        std::process::exit(1);
    }
    println!(
        "Wayward Wrangler scripted UI detached. Reopen with the same --data-dir to inspect this demonstration."
    );
}
