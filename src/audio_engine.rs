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
    pub sample_ptr: usize,             // Tracks how many samples we've consumed
}
unsafe impl Send for Voice {}
unsafe impl Sync for Voice {}

pub struct VoiceManager {
    pub voices: Vec<Voice>,
}

impl VoiceManager {
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
        Self { voices }
    }
    
    pub fn set_sample_rate(&mut self, sr: usize) {
        for v in &mut self.voices {
            v.engine.set_sr(sr);
        }
    }

    pub fn allocate(&mut self, note: u8, code: &str) {
        if let Some(v) = self.voices.iter_mut().find(|v| v.state == VoiceState::Free) {
            v.state = VoiceState::Playing { note };
            v.engine.update_with_code(code);
        }
    }
    
    pub fn release(&mut self, note: u8) {
        for v in &mut self.voices {
            if let VoiceState::Playing { note: active_note } = v.state {
                if active_note == note {
                    v.state = VoiceState::Releasing { note, fade_multiplier: 1.0 };
                }
            }
        }
    }
}
