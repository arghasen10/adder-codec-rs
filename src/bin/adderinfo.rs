use adder_codec_rs::framer::event_framer::EventCoordless;
use adder_codec_rs::framer::scale_intensity::{event_to_intensity, eventcoordless_to_intensity};
use adder_codec_rs::raw::raw_stream::RawStream;
use adder_codec_rs::{Codec, Intensity, D_MAX, D_SHIFT};
use clap::ArgAction::SetTrue;
use clap::Parser;
use itertools::min;
use std::io::Write;
use std::path::Path;
use std::{env, io};

/// Command line argument parser
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
pub struct MyArgs {
    /// Input video path
    #[clap(short, long)]
    pub(crate) input: String,

    /// Calculate dynamic range of the event stream? (Takes more time)
    #[clap(short, long, default_value_t = false, action(SetTrue))]
    pub(crate) dynamic_range: bool,
}

fn main() -> Result<(), std::io::Error> {
    let args: MyArgs = MyArgs::parse();
    let file_path = args.input.as_str();

    let mut stream: RawStream = Codec::new();
    stream.open_reader(file_path).expect("Invalid path");
    let header_bytes = stream.decode_header().expect("Invalid header");

    let mut min_event = EventCoordless {
        d: D_MAX,
        delta_t: 0,
    };
    let mut max_event = EventCoordless {
        d: 0,
        delta_t: stream.delta_t_max,
    };
    let mut max_intensity: Intensity = 0.0;
    let mut min_intensity: Intensity = f64::MAX;

    // Calculate (roughly) the dynamic range of the events. That is, what is the highest intensity
    // event, and what is the lowest intensity event?
    if args.dynamic_range {
        loop {
            match stream.decode_event() {
                Ok(event) => match event_to_intensity(&event) {
                    a if event.d == 255 => {
                        // ignore empty events
                    }
                    a if a < min_intensity => {
                        if event.d == 254 {
                            min_intensity = 1.0 / event.delta_t as f64;
                        } else {
                            min_intensity = a;
                        }
                    }
                    a if a > max_intensity => {
                        max_intensity = a;
                    }
                    _ => {}
                },
                Err(_e) => {
                    break;
                }
            }
        }
    }

    let eof_position_bytes = stream.get_eof_position().unwrap();
    let file_size = Path::new(file_path).metadata().unwrap().len();
    let num_events = (eof_position_bytes - 1 - header_bytes) / stream.event_size as usize;
    let events_per_px = num_events / (stream.width as usize * stream.height as usize);

    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout.lock());

    writeln!(handle, "Dimensions")?;
    writeln!(handle, "\tWidth: {}", stream.width)?;
    writeln!(handle, "\tHeight: {}", stream.height)?;
    writeln!(handle, "\tColor channels: {}", stream.channels)?;
    writeln!(handle, "Source camera: {}", stream.source_camera)?;
    writeln!(handle, "ADΔER transcoder parameters")?;
    writeln!(handle, "\tCodec version: {}", stream.codec_version)?;
    writeln!(handle, "\tTicks per second: {}", stream.tps)?;
    writeln!(
        handle,
        "\tReference ticks per source interval: {}",
        stream.ref_interval
    )?;
    writeln!(handle, "\tΔt_max: {}", stream.delta_t_max)?;
    writeln!(handle, "File metadata")?;
    writeln!(handle, "\tFile size: {}", file_size)?;
    writeln!(handle, "\tHeader size: {}", header_bytes)?;
    writeln!(handle, "\tADΔER event count: {}", num_events)?;
    writeln!(handle, "\tEvents per pixel: {}", events_per_px)?;

    if args.dynamic_range {
        let theory_dr_ratio = D_SHIFT[D_SHIFT.len() - 1] as f64 / (1.0 / stream.delta_t_max as f64);
        let theory_dr_db = 10.0 * theory_dr_ratio.log10();
        let theory_dr_bits = theory_dr_ratio.log2();
        writeln!(handle, "Dynamic range")?;
        writeln!(handle, "\tTheoretical range:")?;
        writeln!(handle, "\t\t{} dB (power)", theory_dr_db as u32)?;
        writeln!(handle, "\t\t{} bits", theory_dr_bits as u32)?;

        let real_dr_ratio = max_intensity / min_intensity;
        let real_dr_db = 10.0 * real_dr_ratio.log10();
        let real_dr_bits = real_dr_ratio.log2();
        writeln!(handle, "\tRealized range:")?;
        writeln!(handle, "\t\t{} dB (power)", real_dr_db as u32)?;
        writeln!(handle, "\t\t{} bits", real_dr_bits as u32)?;
    }

    handle.flush().unwrap();
    Ok(())
}
