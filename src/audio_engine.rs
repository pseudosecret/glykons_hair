use glicol::Engine;

#[derive(Clone, Copy, PartialEq)]
pub enum VoiceState {
    Free,
    Playing { note: u8 },
    Releasing { note: u8, fade_multiplier: f32 },
}

pub struct Voice {
    pub id: usize,
    pub engine: Engine<128>,
    pub state: VoiceState,
    pub out_buffers: [[f32; 128]; 2], // Stores the last computed 128-sample block
    pub sample_ptr: usize,            // Tracks how many samples we've consumed
}

#[derive(Clone)]
struct RegisteredSample {
    symbol: String,
    samples: &'static [f32],
    channels: usize,
    sample_rate: usize,
}

pub struct VoiceManager {
    pub voices: Vec<Voice>,
    sample_rate: usize,
    registered_samples: Vec<RegisteredSample>,
}

impl VoiceManager {
    /// This helper creates a clean Glicol engine for a voice while preserving host/runtime setup.
    /// Glicol keeps its previous graph when a delayed live-code update fails, so replacing the
    /// engine on note allocation gives each trigger a clean graph, clean clock, and the same sample
    /// library as the rest of the voice pool.
    fn build_engine(sample_rate: usize, registered_samples: &[RegisteredSample]) -> Engine<128> {
        let mut engine = Engine::<128>::new();
        engine.set_sr(sample_rate);

        for sample in registered_samples {
            engine.add_sample(
                &sample.symbol,
                sample.samples,
                sample.channels,
                sample.sample_rate,
            );
        }

        engine
    }

    /// This helper prepares a voice for fresh playback of a runtime graph.
    /// It centralizes the state reset so MIDI notes, preview notes, and live-code reloads cannot
    /// accidentally reuse stale audio buffers or a stale Glicol graph from an earlier pattern.
    fn reset_voice_for_code(
        voice: &mut Voice,
        sample_rate: usize,
        registered_samples: &[RegisteredSample],
        note: u8,
        code: &str,
    ) {
        voice.engine = Self::build_engine(sample_rate, registered_samples);
        voice.engine.update_with_code(code);
        voice.state = VoiceState::Playing { note };
        voice.out_buffers[0].fill(0.0);
        voice.out_buffers[1].fill(0.0);
        voice.sample_ptr = 128;
    }

    pub fn new(count: usize) -> Self {
        let mut voices = Vec::with_capacity(count);
        for i in 0..count {
            voices.push(Voice {
                id: i,
                engine: Engine::<128>::new(),
                state: VoiceState::Free,
                out_buffers: [[0.0; 128]; 2],
                sample_ptr: 128, // Forces an immediate next_block() call
            });
        }
        Self {
            voices,
            sample_rate: 44_100,
            registered_samples: Vec::new(),
        }
    }

    pub fn set_sample_rate(&mut self, sr: usize) {
        self.sample_rate = sr;
        for v in &mut self.voices {
            v.engine.set_sr(sr);
        }
    }

    /// This registers decoded user samples with every resident Glicol engine.
    /// Voices are preallocated for realtime playback, so samples must be loaded into each engine
    /// before a note can safely trigger code that references the sample symbol.
    pub fn add_sample(
        &mut self,
        symbol: &str,
        samples: &'static [f32],
        channels: usize,
        sample_rate: usize,
    ) {
        if let Some(existing) = self
            .registered_samples
            .iter_mut()
            .find(|sample| sample.symbol == symbol)
        {
            *existing = RegisteredSample {
                symbol: symbol.to_string(),
                samples,
                channels,
                sample_rate,
            };
        } else {
            self.registered_samples.push(RegisteredSample {
                symbol: symbol.to_string(),
                samples,
                channels,
                sample_rate,
            });
        }

        for v in &mut self.voices {
            v.engine.add_sample(symbol, samples, channels, sample_rate);
        }
    }

    /// This method assigns the requested code to the first free voice.
    /// Voice allocation is the point where old and new patterns most visibly meet, so it rebuilds
    /// the underlying engine instead of relying on Glicol's delayed live-code update machinery.
    pub fn allocate(&mut self, note: u8, code: &str) {
        let sample_rate = self.sample_rate;
        let registered_samples = self.registered_samples.clone();
        if let Some(v) = self.voices.iter_mut().find(|v| v.state == VoiceState::Free) {
            Self::reset_voice_for_code(v, sample_rate, &registered_samples, note, code);
        }
    }

    /// This method force-reloads every active voice for a MIDI note with freshly compiled code.
    /// It gives editor changes immediate audible feedback and prevents a failed/stale previous
    /// graph from continuing to run after the text has clearly changed.
    pub fn reload_active_note(&mut self, note: u8, code: &str) {
        let sample_rate = self.sample_rate;
        let registered_samples = self.registered_samples.clone();

        for v in &mut self.voices {
            if let VoiceState::Playing { note: active_note } = v.state {
                if active_note == note {
                    Self::reset_voice_for_code(v, sample_rate, &registered_samples, note, code);
                }
            }
        }
    }

    pub fn release(&mut self, note: u8) {
        for v in &mut self.voices {
            if let VoiceState::Playing { note: active_note } = v.state {
                if active_note == note {
                    v.state = VoiceState::Releasing {
                        note,
                        fade_multiplier: 1.0,
                    };
                }
            }
        }
    }
}
