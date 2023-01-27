#![deny(warnings, clippy::all)]

use clap::Parser;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;
use trace_recorder_parser::streaming::RecorderData;

#[derive(Parser, Debug, Clone)]
#[clap(name = "streaming example", version, about = "Parse streaming data from file", long_about = None)]
pub struct Opts {
    /// Skip parsing the events
    #[clap(long)]
    pub no_events: bool,

    /// Path to streaming data file
    #[clap(value_parser)]
    pub path: PathBuf,
}

fn main() {
    match do_main() {
        Ok(()) => (),
        Err(e) => {
            eprintln!("{e}");
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

    let mut rd = RecorderData::read(&mut f)?;
    println!("{rd:#?}");

    if !opts.no_events {
        let mut observed_type_counters = BTreeMap::new();

        while let Some((event_code, event)) = rd.read_event(&mut f)? {
            let event_type = event_code.event_type();
            println!("{event_type} : {event}");
            *observed_type_counters.entry(event_type).or_insert(0) += 1_u64;
        }

        println!("----------------------------");
        for (t, count) in observed_type_counters.into_iter() {
            println!("  {t} : {count}");
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
