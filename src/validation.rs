#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SyntaxMode {
    Glicol,
    Strudel,
    FoxDot,
    Rhai,
}

pub enum AudioMessage {
    NewSourceText {
        note: u8,
        compiled_code: String,
    },
    LoadSample {
        symbol: String,
        samples: &'static [f32],
        channels: usize,
        sample_rate: usize,
    },
    PreviewNoteOn {
        note: u8,
    },
    PreviewNoteOff {
        note: u8,
    },
    Panic, // Instantly silences all active engines
}

/// This function is the single source-to-runtime compiler boundary for the project.
/// The editor thread calls this before sending updates so the audio thread only receives
/// runtime-ready code and can stay focused on real-time-safe rendering responsibilities.
pub fn compile_source_for_runtime(source_text: &str, mode: SyntaxMode) -> Result<String, String> {
    let sanitized_code = source_text.replace('\r', "");

    match mode {
        SyntaxMode::Glicol => Ok(sanitized_code),
        SyntaxMode::Strudel => Ok(crate::translator::translate_strudel(&sanitized_code)),
        SyntaxMode::FoxDot => Ok(crate::translator::translate_foxdot(&sanitized_code)),
        SyntaxMode::Rhai => crate::rhai_engine::evaluate_rhai(&sanitized_code),
    }
}
