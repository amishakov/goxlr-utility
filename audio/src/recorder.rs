use crate::audio::get_input;
use anyhow::Result;
use ebur128::{EbuR128, Mode};
use log::info;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Recorder {
    file: PathBuf,
    device: Option<String>,
    stop: Arc<AtomicBool>,
}

impl Recorder {
    pub fn new(file: &Path, device: Option<String>) -> Result<Self> {
        Ok(Self {
            file: file.to_path_buf(),
            device,
            stop: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn record(&mut self) -> Result<()> {
        // Prep the file writer..
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&self.file, spec)?;

        // Grab the Audio Reader..
        let mut input = get_input(self.device.clone())?;

        // We're going to use the normaliser to ensure that something loud was recorded.
        let mut ebu_r128 = EbuR128::new(2, 48000, Mode::I)?;

        // Being the Read Loop..
        while !self.stop.load(Ordering::Relaxed) {
            if let Ok(samples) = input.read() {
                ebu_r128.add_frames_f32(samples.as_slice())?;

                for sample in samples {
                    writer.write_sample(sample)?;
                }
            }
        }

        // Flush and Finalise the WAV file..
        writer.flush()?;
        writer.finalize()?;

        // Before we do anything else, was any noise recorded?
        if let Ok(loudness) = ebu_r128.loudness_global() {
            if loudness == f64::NEG_INFINITY {
                // No noise received..
                info!("No Noise Received in recording, Cancelling.");
                fs::remove_file(&self.file)?;
            }
        }
        Ok(())
    }

    pub fn get_state(&self) -> RecorderState {
        RecorderState {
            stop: self.stop.clone(),
        }
    }
}

#[derive(Debug)]
pub struct RecorderState {
    pub stop: Arc<AtomicBool>,
}