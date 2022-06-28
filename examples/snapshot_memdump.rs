use clap::Parser;
use std::collections::BTreeSet;
use std::fs::File;
use std::path::PathBuf;
use trace_recorder_parser::snapshot::RecorderData;

/// Parse snapshot data from memory dump file
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Opts {
    /// Skip parsing the events
    #[clap(long)]
    pub no_events: bool,

    /// Path to memory dump file
    #[clap(value_parser)]
    pub path: PathBuf,
}

fn main() {
    match do_main() {
        Ok(()) => (),
        Err(e) => {
            eprintln!("{}", e);
            let mut cause = e.source();
            while let Some(err) = cause {
                eprintln!("Caused by: {err}");
                cause = err.source();
            }
            std::process::exit(exitcode::SOFTWARE);
        }
    }
}

fn do_main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();

    reset_signal_pipe_handler()?;

    try_init_tracing_subscriber()?;

    let mut f = File::open(&opts.path)?;
    let desc = RecorderData::locate_and_parse(&mut f)?;
    println!("{:#?}", desc);

    if !opts.no_events {
        let mut types_seen = BTreeSet::new();

        for event in desc.events(&mut f)? {
            let (event_type, event) = event?;
            println!("{event_type} : {event}");
            types_seen.insert(event_type);
        }

        println!("----------------------------");
        for t in types_seen.iter() {
            println!("  {t}");
        }
        println!("----------------------------");
    }

    Ok(())
}

fn try_init_tracing_subscriber() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tracing_subscriber::fmt::Subscriber::builder();
    let env_filter = std::env::var(tracing_subscriber::EnvFilter::DEFAULT_ENV)
        .map(tracing_subscriber::EnvFilter::new)
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(format!(
                "{}={}",
                env!("CARGO_PKG_NAME").replace('-', "_"),
                tracing::Level::WARN
            ))
        });
    let builder = builder.with_env_filter(env_filter);
    let subscriber = builder.finish();
    use tracing_subscriber::util::SubscriberInitExt;
    subscriber.try_init()?;
    Ok(())
}

// Used to prevent panics on broken pipes.
// See:
//   https://github.com/rust-lang/rust/issues/46016#issuecomment-605624865
fn reset_signal_pipe_handler() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_family = "unix")]
    {
        use nix::sys::signal;

        unsafe {
            signal::signal(signal::Signal::SIGPIPE, signal::SigHandler::SigDfl)?;
        }
    }

    Ok(())
}
