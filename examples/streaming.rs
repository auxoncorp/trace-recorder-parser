use clap::Parser;
use std::collections::BTreeMap;
use std::{fs::File, io::BufReader, path::PathBuf};
use trace_recorder_parser::streaming::{Error, RecorderData};
use tracing::{error, warn};

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

    tracing_subscriber::fmt::init();

    let f = File::open(&opts.path)?;
    let mut r = BufReader::new(f);

    let mut rd = RecorderData::find(&mut r)?;
    println!("{rd:#?}");

    if !opts.no_events {
        let mut observed_type_counters = BTreeMap::new();

        loop {
            let (event_code, event) = match rd.read_event(&mut r) {
                Ok(Some((ec, ev))) => (ec, ev),
                Ok(None) => break,
                Err(e) => match e {
                    Error::TraceRestarted(psf_start_word_endianness) => {
                        warn!("Detected a restarted trace stream");
                        rd = RecorderData::read_with_endianness(psf_start_word_endianness, &mut r)?;
                        continue;
                    }
                    _ => {
                        error!("{e}");
                        continue;
                    }
                },
            };

            let event_type = event_code.event_type();
            println!("{event_type} : {event} : {}", event.event_count());
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
