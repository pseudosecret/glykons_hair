use nih_plug::prelude::*;
use std::sync::Arc;

pub mod audio_engine;
pub mod editor;
pub mod params;
pub mod pattern_preview;
pub mod rhai_engine;
pub mod samples;
pub mod timbres;
pub mod translator;
pub mod validation;

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
    const URL: &'static str = "https://github.com/pseudosecret/glykons_hair";
    const EMAIL: &'static str = "pseudosecret@users.noreply.github.com";

    const VERSION: &'static str = "1.0.0";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: std::num::NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

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
        self.voice_manager
            .set_sample_rate(buffer_config.sample_rate as usize);
        let persisted_samples = self
            .params
            .user_samples
            .read()
            .map(|library| library.samples.clone())
            .unwrap_or_default();

        for sample in persisted_samples {
            if let Ok(loaded_sample) =
                samples::load_wav_sample(&sample.id, std::path::Path::new(&sample.path))
            {
                self.voice_manager.add_sample(
                    &loaded_sample.symbol,
                    loaded_sample.samples,
                    loaded_sample.channels,
                    loaded_sample.sample_rate,
                );
            }
        }

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
                validation::AudioMessage::NewSourceText {
                    note,
                    compiled_code,
                } => {
                    self.valid_code_map[note as usize] = compiled_code.clone();
                    self.voice_manager.reload_active_note(note, &compiled_code);
                }
                validation::AudioMessage::LoadSample {
                    symbol,
                    samples,
                    channels,
                    sample_rate,
                } => {
                    self.voice_manager
                        .add_sample(&symbol, samples, channels, sample_rate);
                }
                validation::AudioMessage::PreviewNoteOn { note } => {
                    let code = self.valid_code_map[note as usize].clone();
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
                }
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
                        _ => (),
                    }
                }
                next_event = context.next_event();
                continue; // Re-evaluate loop with new event
            }

            // Now mix `chunk_size` samples for all active voices
            for v in &mut self.voice_manager.voices {
                if v.state == audio_engine::VoiceState::Free {
                    continue;
                }

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
                        audio_engine::VoiceState::Releasing {
                            note,
                            mut fade_multiplier,
                        } => {
                            fade_multiplier -= 0.0001 * to_process as f32; // ~10,000 samples to fade out
                            if fade_multiplier <= 0.0 {
                                is_free = true;
                                0.0
                            } else {
                                v.state = audio_engine::VoiceState::Releasing {
                                    note,
                                    fade_multiplier,
                                };
                                fade_multiplier
                            }
                        }
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
                                    out_ch[block_ptr + v_ptr + i] +=
                                        src_ch[v.sample_ptr + i] * 0.5 * fade_val;
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
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer];
}

impl Vst3Plugin for GlykonsHair {
    const VST3_CLASS_ID: [u8; 16] = *b"GlykonHair_12345";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(GlykonsHair);
nih_export_vst3!(GlykonsHair);

#[cfg(test)]
mod tests {
    use crate::timbres::TIMBRE_NAMES;
    use crate::validation::{compile_source_for_runtime, validate_glicol_code, SyntaxMode};
    use glicol::Engine;

    /// This helper renders a short Glicol graph and returns its peak absolute sample value.
    /// Graph validation only proves that Glicol accepted the syntax, while this protects the
    /// musical contract that translated patterns actually produce audible audio.
    fn render_peak(code: &str, blocks: usize) -> f32 {
        let mut engine = Engine::<128>::new();
        engine.update_with_code(code);

        let mut peak = 0.0_f32;
        for _ in 0..blocks {
            let (out, logs) = engine.next_block(vec![]);
            assert_eq!(logs[0], 0, "Glicol reported an error while rendering");

            for channel in out {
                for sample in channel.iter() {
                    peak = peak.max(sample.abs());
                }
            }
        }

        peak
    }

    #[test]
    // This test protects the core engine integration contract by ensuring a valid graph
    // produces a full 128-sample output block, which is the runtime block size assumption.
    fn glicol_graph_generates_audio() {
        let mut engine = Engine::<128>::new();
        engine.update_with_code(
            "
~trigger: speed 4.0 >> seq 60 _ 60 _ 
~env: ~trigger >> envperc 0.01 0.4
~pitch: ~env >> mul 150 >> add 50
~kick: sin ~pitch >> mul ~env 

~bass_seq: speed 4.0 >> seq _ 40 _ 40
~bass: saw ~bass_seq >> lpf 800 1.0 >> mul 0.3

out: ~kick >> add ~bass >> mul 0.5
        ",
        );
        let (out, _logs) = engine.next_block(vec![]);
        assert!(!out.is_empty(), "Expected at least one output channel");
        assert_eq!(out[0].len(), 128, "Expected one full audio block");
    }

    #[test]
    // This test verifies that Strudel input compiles into a graph containing both a trigger lane
    // and an output lane, which are required for note sequencing to reach the audio output.
    fn strudel_compile_produces_runtime_graph() {
        let compiled =
            compile_source_for_runtime("note(\"c3 e3 g3\").s(\"sawbass\")", SyntaxMode::Strudel)
                .expect("Strudel translation should compile");
        assert!(
            compiled.contains("out:"),
            "Translated graph should include output node"
        );
        assert!(
            compiled.contains("~p1_trig"),
            "Translated graph should include trigger sequence"
        );
    }

    #[test]
    // This test reproduces the reported Element failure: a Strudel sawbass pattern can compile but
    // still be silent if the translator wires Glicol's sequencer ratios into oscillators wrongly.
    fn strudel_sawbass_renders_audible_audio() {
        let compiled =
            compile_source_for_runtime("note(\"c3\").s(\"sawbass\")", SyntaxMode::Strudel)
                .expect("Strudel translation should compile");

        let peak = render_peak(&compiled, 64);
        assert!(peak > 0.001, "Expected audible Strudel output, got {peak}");
    }

    #[test]
    // This test keeps the forgiving text extractor useful for Strudel-ish reversed chains.
    // The prototype translator scans for both function calls, so either order should become sound.
    fn strudel_reversed_chain_renders_audible_audio() {
        let compiled =
            compile_source_for_runtime("s(\"sawbass\").note(\"c3\")", SyntaxMode::Strudel)
                .expect("Reversed Strudel-like chain should compile");

        let peak = render_peak(&compiled, 64);
        assert!(
            peak > 0.001,
            "Expected audible reversed-chain Strudel output, got {peak}"
        );
    }

    #[test]
    // This test protects the central Strudel goal: multiple `$:` pattern lanes should compile into
    // independent Glicol chains and be summed, rather than overwriting each other or going silent.
    fn strudel_parallel_dollar_patterns_render_audio() {
        let source = r#"
$: note("<[c2 c3]*4 [bb1 bb2]*4 [f2 f3]*4 [eb2 eb3]*4>")
.sound("gm_synth_bass_1").lpf(800)

$: n(`<
[~ 0] 2 [0 2] [~ 2]
[~ 0] 1 [0 1] [~ 1]
[~ 0] 3 [0 3] [~ 3]
[~ 0] 2 [0 2] [~ 2]
>*4`).scale("C4:minor")
.sound("gm_synth_strings_1")
"#;
        let compiled = compile_source_for_runtime(source, SyntaxMode::Strudel)
            .expect("Parallel Strudel patterns should compile");

        assert!(
            compiled.contains("~p1_final") && compiled.contains("~p2_final"),
            "Expected two independent pattern lanes, got:\n{compiled}"
        );
        assert!(
            compiled.contains("out: ~p1_final >> add ~p2_final"),
            "Expected both pattern lanes to be mixed, got:\n{compiled}"
        );

        let peak = render_peak(&compiled, 128);
        assert!(
            peak > 0.001,
            "Expected audible parallel Strudel output, got {peak}"
        );
    }

    #[test]
    // This test encodes the UX rule from the editor examples: if users write more than one Strudel
    // pattern line, each lane must be prefixed with `$:` so the translator can split them safely.
    fn strudel_multiple_patterns_without_dollar_report_error() {
        let source = "note(\"c3 e3 g3\")\nnote(\"e3 g3 c4\")";
        let result = compile_source_for_runtime(source, SyntaxMode::Strudel);
        let error = result.expect_err("Missing `$:` should report a Strudel error");
        assert!(
            error.contains("$:"),
            "Expected `$:` guidance in error message, got: {error}"
        );
    }

    #[test]
    // This test catches the malformed repeat case where `*` appears outside the pattern string.
    // The translator should reject it explicitly instead of silently ignoring the suffix.
    fn strudel_repeat_outside_pattern_string_reports_error() {
        let source = "$: note(\"<c2>\")*3";
        let result = compile_source_for_runtime(source, SyntaxMode::Strudel);
        let error = result.expect_err("Repeat outside quotes should be rejected");
        assert!(
            error.contains("inside"),
            "Expected repeat-placement error message, got: {error}"
        );
    }

    #[test]
    // This test protects grouped rhythm handling: `9 [1 1]` should become a 2:1:1 onset pattern,
    // which maps to `9 _ 1 1` in the sequencer lane instead of three evenly spaced onsets.
    fn strudel_grouped_rhythm_expands_with_rest_holds() {
        let compiled = compile_source_for_runtime("$: note(\"9 [1 1]\")", SyntaxMode::Strudel)
            .expect("Grouped Strudel rhythm should compile");
        assert!(
            compiled.contains("seq 76 _ 62 62"),
            "Expected grouped rhythm expansion in compiled seq, got:\n{compiled}"
        );
    }

    #[test]
    // This test covers the README promise that FoxDot-style input can be selected and translated.
    // It does not claim full FoxDot compatibility, but it protects the supported player/synth form.
    fn foxdot_compile_produces_runtime_graph() {
        let compiled =
            compile_source_for_runtime("p1 >> tb303([0, 2, 4], dur=2)", SyntaxMode::FoxDot)
                .expect("FoxDot translation should compile");
        assert!(
            compiled.contains("out:"),
            "Translated graph should include output node"
        );
        assert!(
            compiled.contains("~p1_trig"),
            "Translated graph should include trigger sequence"
        );
        assert!(compiled.contains("lpf"), "TB-303 timbre should be selected");
    }

    #[test]
    // This test walks every advertised timbre through the Strudel translator and Glicol validator.
    // The editor exposes these names as clickable presets, so each one must build a real runtime
    // graph instead of silently leaving the previous sound alive.
    fn every_advertised_timbre_builds_a_valid_graph() {
        for timbre in TIMBRE_NAMES {
            let source = format!("note(\"c3\").s(\"{timbre}\")");
            let compiled = compile_source_for_runtime(&source, SyntaxMode::Strudel)
                .unwrap_or_else(|err| panic!("{timbre} should compile: {err}"));

            validate_glicol_code(&compiled)
                .unwrap_or_else(|err| panic!("{timbre} should validate: {err}"));
        }
    }

    #[test]
    // This test ensures syntax failures propagate back as errors instead of silently replacing
    // runtime code with empty output, protecting live-coding feedback behavior.
    fn rhai_compile_reports_errors() {
        let result = compile_source_for_runtime("note(\"c3\").fast(", SyntaxMode::Rhai);
        assert!(result.is_err(), "Invalid Rhai should return an error");
    }
}
