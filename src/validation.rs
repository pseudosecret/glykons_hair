use glicol::{Engine, EngineError};

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

/// This function formats Glicol's low-level engine errors for editor feedback.
/// The plugin needs concise messages because invalid live-code should be visible to the user
/// before the audio thread has a chance to keep playing a stale graph.
fn format_glicol_error(error: EngineError) -> String {
    match error {
        EngineError::ParsingError(error) => format!("Glicol parse error: {error}"),
        EngineError::NonExistReference(reference) => {
            format!("Glicol reference does not exist: {reference}")
        }
        EngineError::NonExsitSample(sample) => {
            format!("Glicol sample has not been loaded into the scratch validator: {sample}")
        }
    }
}

/// This function verifies that runtime Glicol code can build a graph before playback sees it.
/// Glicol applies `update_with_code()` lazily during rendering and leaves the old graph alive on
/// failure, so editor-side validation prevents stale sounds from surviving a failed translation.
pub fn validate_glicol_code(code: &str) -> Result<(), String> {
    let mut engine = Engine::<128>::new();
    engine.update_with_code(code);

    match engine.update() {
        Ok(()) => Ok(()),
        Err(EngineError::NonExsitSample(_)) => Ok(()),
        Err(error) => Err(format_glicol_error(error)),
    }
}

/// This function is the single source-to-runtime compiler boundary for the project.
/// The editor thread calls this before sending updates so the audio thread only receives
/// runtime-ready code and can stay focused on real-time-safe rendering responsibilities.
pub fn compile_source_for_runtime(source_text: &str, mode: SyntaxMode) -> Result<String, String> {
    let sanitized_code = source_text.replace('\r', "");

    let compiled_code = match mode {
        SyntaxMode::Glicol => Ok(sanitized_code),
        SyntaxMode::Strudel => crate::translator::translate_strudel(&sanitized_code),
        SyntaxMode::FoxDot => Ok(crate::translator::translate_foxdot(&sanitized_code)),
        SyntaxMode::Rhai => crate::rhai_engine::evaluate_rhai(&sanitized_code),
    }?;

    validate_glicol_code(&compiled_code)?;

    Ok(compiled_code)
}
