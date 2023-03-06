use std::error::Error;

use adder_codec_core::DeltaT;
use adder_codec_rs::transcoder::source::davis::Davis;
use adder_codec_rs::transcoder::source::framed::Framed;
use bevy::prelude::Image;
use std::fmt;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use adder_codec_rs::transcoder::source::davis::TranscoderMode;

use adder_codec_rs::aedat::base::ioheader_generated::Compression;
use adder_codec_rs::davis_edi_rs::util::reconstructor::Reconstructor;

use crate::transcoder::ui::{ParamsUiState, TranscoderState};
use adder_codec_core::SourceCamera::{DavisU8, FramedU8};
use adder_codec_core::TimeMode;
use adder_codec_rs::transcoder::source::video::VideoBuilder;
use bevy_egui::egui::{Color32, RichText};
use opencv::Result;

pub struct AdderTranscoder {
    pub(crate) framed_source: Option<Framed<BufWriter<File>>>,
    pub(crate) davis_source: Option<Davis<BufWriter<File>>>,
    pub(crate) live_image: Image,
}

impl Default for AdderTranscoder {
    fn default() -> Self {
        Self {
            framed_source: None,
            davis_source: None,
            live_image: Image::default(),
        }
    }
}

#[derive(Debug)]
struct AdderTranscoderError(String);

impl fmt::Display for AdderTranscoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ADDER transcoder: {}", self.0)
    }
}

impl Error for AdderTranscoderError {}

impl AdderTranscoder {
    pub(crate) fn new(
        input_path_buf: &Path,
        input_path_buf_1: &Option<PathBuf>,
        output_path_opt: Option<PathBuf>,
        ui_state: &mut ParamsUiState,
        current_frame: u32,
    ) -> Result<Self, Box<dyn Error>> {
        match input_path_buf.extension() {
            None => Err(Box::new(AdderTranscoderError("Invalid file type".into()))),
            Some(ext) => {
                match ext.to_ascii_lowercase().to_str() {
                    None => Err(Box::new(AdderTranscoderError("Invalid file type".into()))),
                    Some("mp4") => {
                        let mut framed: Framed<BufWriter<File>> = Framed::new(
                            match input_path_buf.to_str() {
                                None => {
                                    return Err(Box::new(AdderTranscoderError(
                                        "Couldn't get input path string".into(),
                                    )))
                                }
                                Some(path) => path.to_string(),
                            },
                            ui_state.color,
                            ui_state.scale,
                        )?
                        .frame_start(current_frame)?
                        .chunk_rows(64)
                        .c_thresh_pos(ui_state.adder_tresh as u8)
                        .c_thresh_neg(ui_state.adder_tresh as u8)
                        .auto_time_parameters(
                            ui_state.delta_t_ref as u32,
                            ui_state.delta_t_max_mult * ui_state.delta_t_ref as u32,
                        )?
                        .time_mode(ui_state.time_mode)
                        .show_display(false);

                        // TODO: Change the builder to take in a pathbuf directly, not a string,
                        // and to handle the error checking in the associated function
                        match output_path_opt {
                            None => {}
                            Some(output_path) => {
                                let out_path = output_path.to_str().unwrap();
                                let writer = BufWriter::new(File::create(out_path)?);
                                framed = *framed.write_out(FramedU8, ui_state.time_mode, writer)?;
                                //     .output_events_filename(match output_path.to_str() {
                                //     None => {
                                //         return Err(Box::new(AdderTranscoderError(
                                //             "Couldn't get output path string".into(),
                                //         )))
                                //     }
                                //     Some(path) => path.parse()?,
                                // });
                            }
                        };

                        ui_state.delta_t_ref_max = 255.0;
                        Ok(AdderTranscoder {
                            framed_source: Some(framed),
                            davis_source: None,
                            live_image: Default::default(),
                        })
                        // }
                        // Err(_e) => {
                        //     Err(Box::new(AdderTranscoderError("Invalid file type".into())))
                        // }
                    }

                    Some(ext) if ext == "aedat4" || ext == "sock" => {
                        let events_only = match &ui_state.davis_mode_radio_state {
                            TranscoderMode::Framed => false,
                            TranscoderMode::RawDavis => false,
                            TranscoderMode::RawDvs => true,
                        };
                        let deblur_only = match &ui_state.davis_mode_radio_state {
                            TranscoderMode::Framed => false,
                            TranscoderMode::RawDavis => true,
                            TranscoderMode::RawDvs => true,
                        };

                        let rt = tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(ui_state.thread_count)
                            .enable_time()
                            .build()?;
                        let dir = input_path_buf
                            .parent()
                            .expect("File must be in some directory")
                            .to_str()
                            .expect("Bad path")
                            .to_string();
                        let filename_0 = input_path_buf
                            .file_name()
                            .expect("File must exist")
                            .to_str()
                            .expect("Bad filename")
                            .to_string();
                        eprintln!("{filename_0}");

                        let mode = match ext {
                            "aedat4" => "file",
                            "sock" => "socket",
                            _ => "file",
                        };

                        let filename_1 = match input_path_buf_1 {
                            None => None,
                            Some(input_path_buf_1) => Some(
                                input_path_buf_1
                                    .file_name()
                                    .expect("File must exist")
                                    .to_str()
                                    .expect("Bad filename")
                                    .to_string(),
                            ),
                        };
                        dbg!(filename_1.clone());

                        let reconstructor = rt.block_on(Reconstructor::new(
                            dir + "/",
                            filename_0,
                            filename_1.unwrap_or("".to_string()),
                            mode.to_string(), // TODO
                            0.15,
                            ui_state.optimize_c,
                            false,
                            false,
                            false,
                            ui_state.davis_output_fps,
                            Compression::None,
                            346,
                            260,
                            deblur_only,
                            events_only,
                            1000.0, // Target latency (not used)
                            true,
                        ));

                        let output_string = output_path_opt
                            .map(|output_path| output_path.to_str().expect("Bad path").to_string());

                        let mut davis_source: Davis<BufWriter<File>> =
                            Davis::new(reconstructor, rt)?
                                .optimize_adder_controller(false) // TODO
                                .mode(ui_state.davis_mode_radio_state)
                                .time_mode(ui_state.time_mode)
                                .time_parameters(
                                    1000000_u32, // TODO
                                    (1_000_000.0 / ui_state.davis_output_fps) as DeltaT,
                                    (1_000_000.0 * ui_state.delta_t_max_mult as f32) as u32, // TODO
                                )? // TODO
                                .c_thresh_pos(ui_state.adder_tresh as u8)
                                .c_thresh_neg(ui_state.adder_tresh as u8);

                        if let Some(output_string) = output_string {
                            let writer = BufWriter::new(File::create(&output_string)?);
                            davis_source =
                                *davis_source.write_out(DavisU8, TimeMode::DeltaT, writer)?;
                        }

                        Ok(AdderTranscoder {
                            framed_source: None,
                            davis_source: Some(davis_source),
                            live_image: Default::default(),
                        })
                    }

                    Some(_) => Err(Box::new(AdderTranscoderError("Invalid file type".into()))),
                }
            }
        }
    }
}

pub(crate) fn replace_adder_transcoder(
    transcoder_state: &mut TranscoderState,
    input_path_buf_0: Option<PathBuf>,
    input_path_buf_1: Option<PathBuf>,
    output_path_opt: Option<PathBuf>,
    current_frame: u32,
) {
    let mut ui_info_state = &mut transcoder_state.ui_info_state;
    ui_info_state.events_per_sec = 0.0;
    ui_info_state.events_ppc_total = 0.0;
    ui_info_state.events_total = 0;
    ui_info_state.events_ppc_per_sec = 0.0;
    if let Some(input_path) = input_path_buf_0 {
        match AdderTranscoder::new(
            &input_path,
            &input_path_buf_1,
            output_path_opt.clone(),
            &mut transcoder_state.ui_state,
            current_frame,
        ) {
            Ok(transcoder) => {
                eprintln!("bgood");
                transcoder_state.transcoder = transcoder;
                ui_info_state.source_name = RichText::new(
                    input_path
                        .to_str()
                        .unwrap_or("Error: invalid source string"),
                )
                .color(Color32::DARK_GREEN);
                if let Some(output_path) = output_path_opt {
                    ui_info_state.output_name.text = RichText::new(
                        output_path
                            .to_str()
                            .unwrap_or("Error: invalid output string"),
                    )
                    .color(Color32::DARK_GREEN);
                }
                eprintln!("bgood2");
            }
            Err(e) => {
                eprintln!("berror");
                transcoder_state.transcoder = AdderTranscoder::default();
                ui_info_state.source_name = RichText::new(e.to_string()).color(Color32::RED);
            }
        };
    } else {
        eprintln!("No input path");
    }
}
