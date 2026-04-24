use std::path::Path;

pub struct LoadedSample {
    pub symbol: String,
    pub samples: &'static [f32],
    pub channels: usize,
    pub sample_rate: usize,
}

/// This function normalizes user-facing sample IDs into Glicol sample symbols.
/// Glicol's sampler nodes expect symbols such as `\kick`, while the editor lets users type the
/// friendlier `kick` form and keeps the conversion centralized.
pub fn sample_symbol_from_id(id: &str) -> Result<String, String> {
    let trimmed = id.trim().trim_start_matches('\\');
    if trimmed.is_empty() {
        return Err("Sample ID cannot be empty".to_string());
    }

    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(
            "Sample ID may only use letters, numbers, underscores, and hyphens".to_string(),
        );
    }

    Ok(format!("\\{trimmed}"))
}

/// This function loads a WAV file into the static sample representation required by Glicol.
/// The buffer is intentionally leaked for the plugin instance lifetime so the audio engines can
/// hold stable references without copying sample data on the realtime thread.
pub fn load_wav_sample(id: &str, path: &Path) -> Result<LoadedSample, String> {
    let symbol = sample_symbol_from_id(id)?;
    let mut reader =
        hound::WavReader::open(path).map_err(|err| format!("Could not open WAV file: {err}"))?;
    let spec = reader.spec();

    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("Could not read float samples: {err}"))?,
        hound::SampleFormat::Int => {
            let max_amplitude = (1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / max_amplitude)
                        .map_err(|err| format!("Could not read integer samples: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };

    if samples.is_empty() {
        return Err("WAV file contains no samples".to_string());
    }

    let leaked_samples = Box::leak(samples.into_boxed_slice());
    Ok(LoadedSample {
        symbol,
        samples: leaked_samples,
        channels: spec.channels as usize,
        sample_rate: spec.sample_rate as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // This test protects the user-sample naming contract between the editor and Glicol.
    // Users can type friendly IDs, while runtime code receives the backslash-prefixed symbols
    // required by sampler nodes.
    fn sample_symbol_normalizes_user_ids() {
        assert_eq!(sample_symbol_from_id("kick").unwrap(), "\\kick");
        assert_eq!(sample_symbol_from_id("\\snare").unwrap(), "\\snare");
        assert!(sample_symbol_from_id("bad id").is_err());
    }

    #[test]
    // This test verifies the first real custom-sample path: a WAV file can be decoded into the
    // static sample buffer shape the audio engines consume.
    fn wav_loader_decodes_sample_metadata() {
        let path = std::env::temp_dir().join("glykons_hair_sample_test.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        {
            let mut writer = hound::WavWriter::create(&path, spec).unwrap();
            writer.write_sample::<i16>(0).unwrap();
            writer.write_sample::<i16>(i16::MAX / 2).unwrap();
            writer.finalize().unwrap();
        }

        let loaded = load_wav_sample("test", &path).unwrap();
        assert_eq!(loaded.symbol, "\\test");
        assert_eq!(loaded.channels, 1);
        assert_eq!(loaded.sample_rate, 44_100);
        assert_eq!(loaded.samples.len(), 2);
    }
}
