#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SyntaxMode {
    Glicol,
    Strudel,
    FoxDot,
    Rhai,
}

pub enum AudioMessage {
    NewSourceText { note: u8, valid_code: String, mode: SyntaxMode },
    PreviewNoteOn { note: u8 },
    PreviewNoteOff { note: u8 },
    Panic, // Instantly silences all active engines
}
