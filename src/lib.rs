use nih_plug::prelude::*;
use std::sync::Arc;

pub mod params;
pub mod audio_engine;
pub mod validation;
pub mod editor;
pub mod timbres;
pub mod translator;
pub mod rhai_engine;

use crate::params::GlykonsHairParams;
use crossbeam::channel::{Receiver, Sender};

pub struct GlykonsHair {
    params: Arc<GlykonsHairParams>,
    voice_manager: audio_engine::VoiceManager,
    tx: Sender<validation::AudioMessage>,
    rx: Receiver<validation::AudioMessage>,
    valid_code_map: [String; 128],
    previous_pos: i64,
}

impl Default for GlykonsHair {
    fn default() -> Self {
        let (tx, rx) = crossbeam::channel::bounded(256);
        // Default pattern for all keys is a saw
        let valid_code_map: [String; 128] = core::array::from_fn(|_| "out: saw 220".to_string());
        
        Self {
            params: Arc::new(GlykonsHairParams::default()),
            voice_manager: audio_engine::VoiceManager::new(16),
            tx,
            rx,
            valid_code_map,
            previous_pos: 0,
        }
    }
}

impl Plugin for GlykonsHair {
    const NAME: &'static str = "Glykon's Hair";
    const VENDOR: &'static str = "Glykon";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "you@example.com";

    const VERSION: &'static str = "1.0.0";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: std::num::NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        }
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.tx.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.voice_manager.set_sample_rate(buffer_config.sample_rate as usize);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Drain any incoming validated patterns from the GUI
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                validation::AudioMessage::NewSourceText { note, valid_code, mode } => {
                    let sanitized_code = valid_code.replace("\r", "");
                    
                    let final_code = match mode {
                        validation::SyntaxMode::Strudel => translator::translate_strudel(&sanitized_code),
                        validation::SyntaxMode::FoxDot => translator::translate_foxdot(&sanitized_code),
                        validation::SyntaxMode::Rhai => {
                            match rhai_engine::evaluate_rhai(&sanitized_code) {
                                Ok(code) => code,
                                Err(_) => "".to_string(), // In a real app we'd log the error to the UI
                            }
                        },
                        validation::SyntaxMode::Glicol => sanitized_code.clone(),
                    };

                    self.valid_code_map[note as usize] = final_code.clone();
                    // Live Coding Fix: Instantly update any currently playing voices!
                    for v in &mut self.voice_manager.voices {
                        if let audio_engine::VoiceState::Playing { note: active_note } = v.state {
                            if active_note == note {
                                v.engine.update_with_code(&final_code);
                            }
                        }
                    }
                }
                validation::AudioMessage::PreviewNoteOn { note } => {
                    let code = self.valid_code_map[note as usize].clone();
                    nih_log!("Previewing Note {} with code: {}", note, code);
                    self.voice_manager.allocate(note, &code);
                }
                validation::AudioMessage::PreviewNoteOff { note } => {
                    self.voice_manager.release(note);
                }
                validation::AudioMessage::Panic => {
                    for v in &mut self.voice_manager.voices {
                        v.state = audio_engine::VoiceState::Free;
                        v.out_buffers[0].fill(0.0);
                        v.out_buffers[1].fill(0.0);
                    }
                }
            }
        }

        let transport = context.transport();
        let pos = transport.pos_samples().unwrap_or(0);
        
        // Host Transport Rephasing (6.2)
        if (pos - self.previous_pos).abs() > 4096 && self.previous_pos != 0 {
            // Leap detected, silence all voices
            for v in &mut self.voice_manager.voices {
                v.state = audio_engine::VoiceState::Free;
            }
        }
        self.previous_pos = pos + buffer.samples() as i64;

        let num_samples = buffer.samples();
        let mut block_ptr = 0;
        let mut next_event = context.next_event();

        // Zero the canvas ONCE globally
        for out_channel in buffer.as_slice().iter_mut() {
            out_channel.fill(0.0);
        }

        while block_ptr < num_samples {
            // Find the timing of the next event
            let next_event_timing = match &next_event {
                Some(event) => {
                    match event {
                        NoteEvent::NoteOn { timing, .. } => *timing as usize,
                        NoteEvent::NoteOff { timing, .. } => *timing as usize,
                        NoteEvent::Choke { timing, .. } => *timing as usize,
                        NoteEvent::PolyModulation { timing, .. } => *timing as usize,
                        NoteEvent::MonoAutomation { timing, .. } => *timing as usize,
                        NoteEvent::PolyPressure { timing, .. } => *timing as usize,
                        _ => num_samples, // Ignore other events for sample-accurate timing
                    }
                },
                None => num_samples,
            };

            // Calculate safe chunk size before next event
            let mut chunk_size = num_samples - block_ptr;
            if next_event_timing > block_ptr && next_event_timing < num_samples {
                chunk_size = std::cmp::min(chunk_size, next_event_timing - block_ptr);
            } else if next_event_timing == block_ptr {
                // Time to process the event!
                if let Some(event) = next_event.take() {
                    match event {
                        NoteEvent::NoteOn { note, .. } => {
                            let code = self.valid_code_map[note as usize].clone();
                            self.voice_manager.allocate(note, &code);
                        }
                        NoteEvent::NoteOff { note, .. } => {
                            self.voice_manager.release(note);
                        }
                        _ => ()
                    }
                }
                next_event = context.next_event();
                continue; // Re-evaluate loop with new event
            }

            // Now mix `chunk_size` samples for all active voices
            for v in &mut self.voice_manager.voices {
                if v.state == audio_engine::VoiceState::Free { continue; }
                
                let mut v_ptr = 0;
                while v_ptr < chunk_size {
                    // Refill buffer if empty
                    if v.sample_ptr >= 128 {
                        let (glicol_out, _) = v.engine.next_block(vec![]);
                        for c in 0..2 {
                            let src_idx = c.min(glicol_out.len().saturating_sub(1));
                            if glicol_out.len() > 0 && glicol_out[src_idx].len() == 128 {
                                v.out_buffers[c].copy_from_slice(&glicol_out[src_idx]);
                            } else {
                                v.out_buffers[c].fill(0.0);
                            }
                        }
                        v.sample_ptr = 0;
                    }

                    // Consume samples
                    let available = 128 - v.sample_ptr;
                    let to_process = std::cmp::min(available, chunk_size - v_ptr);

                    let mut is_free = false;
                    let fade_val = match v.state {
                        audio_engine::VoiceState::Releasing { note, mut fade_multiplier } => {
                            fade_multiplier -= 0.0001 * to_process as f32; // ~10,000 samples to fade out
                            if fade_multiplier <= 0.0 {
                                is_free = true;
                                0.0
                            } else {
                                v.state = audio_engine::VoiceState::Releasing { note, fade_multiplier };
                                fade_multiplier
                            }
                        },
                        audio_engine::VoiceState::Playing { .. } => 1.0,
                        _ => 0.0,
                    };

                    if is_free {
                        v.state = audio_engine::VoiceState::Free;
                        break;
                    }

                    if fade_val > 0.0 {
                        for (c, out_ch) in buffer.as_slice().iter_mut().enumerate() {
                            if c < 2 {
                                let src_ch = &v.out_buffers[c];
                                for i in 0..to_process {
                                    out_ch[block_ptr + v_ptr + i] += src_ch[v.sample_ptr + i] * 0.5 * fade_val;
                                }
                            }
                        }
                    }

                    v.sample_ptr += to_process;
                    v_ptr += to_process;
                }
            }

            block_ptr += chunk_size;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for GlykonsHair {
    const CLAP_ID: &'static str = "com.glykon.hair";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Glykon's Hair Synth Pattern Launcher");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::Synthesizer];
}

impl Vst3Plugin for GlykonsHair {
    const VST3_CLASS_ID: [u8; 16] = *b"GlykonHair_12345";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(GlykonsHair);
nih_export_vst3!(GlykonsHair);

#[cfg(test)]
mod tests {
    use glicol::Engine;

    #[test]
    fn test_glicol_syntax() {
        let mut engine = Engine::<128>::new();
        engine.update_with_code("
~trigger: speed 4.0 >> seq 60 _ 60 _ 
~env: ~trigger >> envperc 0.01 0.4
~pitch: ~env >> mul 150 >> add 50
~kick: sin ~pitch >> mul ~env 

~bass_seq: speed 4.0 >> seq _ 40 _ 40
~bass: saw ~bass_seq >> lpf 800 1.0 >> mul 0.3

out: ~kick >> add ~bass >> mul 0.5
        ");
        let (out, logs) = engine.next_block(vec![]);
        println!("Output length: {}", out.len());
        // Wait, Engine::<128> next_block might return something else. Let's see.
        assert!(false, "Force fail to read logs");
    }
}
