use clap::Parser;
use std::collections::BTreeMap;
use std::{fs::File, io::BufReader, path::PathBuf};
use tabular::{Row, Table};
use trace_recorder_parser::streaming::{Error, RecorderData};
use tracing::{error, warn};

#[derive(Parser, Debug, Clone)]
#[clap(name = "streaming example", version, about = "Parse streaming data from file", long_about = None)]
pub struct Opts {
    /// Skip parsing the events
    #[clap(long)]
    pub no_events: bool,

    /// TODO
    #[clap(long, value_parser=clap_num::maybe_hex::<u16>)]
    pub custom_printf_event_id: Option<u16>,

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

    if let Some(custom_printf_event_id) = opts.custom_printf_event_id {
        rd.set_custom_printf_event_id(custom_printf_event_id.into());
    }

    println!("{rd:#?}");

    if !opts.no_events {
        let mut observed_type_counters = BTreeMap::new();
        let mut total_count = 0_u64;

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
            total_count += 1;
        }

        println!("--------------------------------------------------------");
        let mut table = Table::new("{:>}    {:>}    {:<}");
        for (handle, entry) in rd.entry_table.entries().iter() {
            let entry_class = if let Some(c) = entry.class {
                c.to_string()
            } else {
                "NA".to_owned()
            };
            let entry_sym = if let Some(s) = &entry.symbol {
                s.as_ref()
            } else {
                "NA"
            };

            table.add_row(
                Row::new()
                    .with_cell(handle)
                    .with_cell(entry_class)
                    .with_cell(entry_sym),
            );
        }
        print!("{table}");

        println!("--------------------------------------------------------");
        let mut table = Table::new("{:>}    {:>}    {:<}");
        for (t, count) in observed_type_counters.into_iter() {
            let percentage = 100.0 * (count as f64 / total_count as f64);
            table.add_row(
                Row::new()
                    .with_cell(count)
                    .with_cell(format!("{percentage:.01}"))
                    .with_cell(t),
            );
        }
        print!("{table}");

        println!("--------------------------------------------------------");
        println!("total: {total_count}");
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
