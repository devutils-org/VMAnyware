mod api;
mod app;
#[cfg(test)]
mod fixtures;
mod model;
mod ui;
mod worker;

use app::App;
use ratatui::crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
};
use std::{
    io::{self, IsTerminal},
    time::Duration,
};
use zeroize::Zeroizing;

const HELP: &str = "VMAnyware · terminal client\n\nUsage: vmanyware tui [--endpoint https://host[:port]] [--ca /path/to/public-ca.pem]\n       vmanyware --help\n\nThe connection screen prompts for an access token without echoing it.\nVMANYWARE_ENDPOINT, VMANYWARE_CA and VMANYWARE_ACCESS_TOKEN are optional.\nOnly HTTPS origins are accepted. An explicit CA restricts trust to that CA.\nNo configuration or token is saved. No VM mutations are available.\n";

fn options(args: impl Iterator<Item = String>) -> Result<Option<(String, String)>, &'static str> {
    let mut args = args.peekable();
    if matches!(args.peek().map(String::as_str), Some("--help" | "-h")) {
        return Ok(None);
    }
    if args.next().as_deref() != Some("tui") {
        return Err("Use 'vmanyware tui' or 'vmanyware --help'.");
    }
    let mut endpoint =
        std::env::var("VMANYWARE_ENDPOINT").unwrap_or_else(|_| "https://localhost".into());
    let mut ca = std::env::var("VMANYWARE_CA").unwrap_or_default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--endpoint" => {
                endpoint = args
                    .next()
                    .filter(|s| !s.starts_with("--"))
                    .ok_or("--endpoint needs an HTTPS origin.")?
            }
            "--ca" => {
                ca = args
                    .next()
                    .filter(|s| !s.starts_with("--"))
                    .ok_or("--ca needs a public CA file path.")?
            }
            _ => return Err("Unknown option. Use 'vmanyware --help'."),
        }
    }
    if endpoint.len() > 4096
        || ca.len() > 4096
        || endpoint.chars().any(char::is_control)
        || ca.chars().any(char::is_control)
    {
        return Err("Endpoint and CA path must be bounded text without control characters.");
    }
    Ok(Some((endpoint, ca)))
}

struct RestoreTerminal;
impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste, DisableMouseCapture);
        ratatui::restore();
    }
}
fn run(endpoint: String, ca: String) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "The TUI requires an interactive terminal on stdin and stdout.",
        ));
    }
    let token = Zeroizing::new(std::env::var("VMANYWARE_ACCESS_TOKEN").unwrap_or_default());
    let mut app = App::new(endpoint, ca, token);
    let (send, replies) = worker::spawn();
    // The guard also restores partially initialized terminals and ordinary I/O errors.
    let _restore = RestoreTerminal;
    let mut terminal = ratatui::try_init()?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableBracketedPaste, DisableMouseCapture);
        previous(info);
    }));
    execute!(io::stdout(), EnableBracketedPaste, EnableMouseCapture)?;
    // Even an environment-supplied token is reviewed on the connection screen;
    // the first request follows Enter, not application startup.
    while !app.quit {
        for reply in replies.try_iter() {
            app.apply(reply);
        }
        app.tick(&send);
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => app.key(key, &send),
                Event::Paste(text) => app.paste(&text),
                Event::Mouse(mouse) => app.mouse(mouse, &send),
                Event::Resize(_, _) => app.hits.clear(),
                _ => (),
            }
        }
    }
    // Do not wait for an outstanding read: dropping the UI never cancels or
    // resubmits a server operation. The process ends the read-only worker.
    Ok(())
}
fn main() {
    match options(std::env::args().skip(1)) {
        Ok(None) => print!("{HELP}"),
        Ok(Some((endpoint, ca))) => {
            if let Err(error) = run(endpoint, ca) {
                eprintln!("VMAnyware: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("VMAnyware: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    #[test]
    fn parser_keeps_tokens_out_of_arguments_and_errors() {
        assert!(options(["--help".into()].into_iter()).unwrap().is_none());
        assert!(options(
            ["tui", "--token", "private-value"]
                .map(String::from)
                .into_iter()
        )
        .unwrap_err()
        .contains("Unknown option"));
        assert!(options(["tui", "--endpoint"].map(String::from).into_iter()).is_err());
        assert!(options(
            ["tui", "--endpoint", "https://host\ncontrol"]
                .map(String::from)
                .into_iter()
        )
        .is_err());
        assert_eq!(
            options(
                ["tui", "--endpoint", "https://host", "--ca", "public.pem"]
                    .map(String::from)
                    .into_iter()
            )
            .unwrap(),
            Some(("https://host".into(), "public.pem".into()))
        );
    }
}
