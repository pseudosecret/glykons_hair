use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

pub struct PatternSlots {
    pub slots: HashMap<u8, String>,
}

impl Default for PatternSlots {
    fn default() -> Self {
        let mut slots = HashMap::new();
        for i in 0..128 {
            slots.insert(i as u8, "out: saw 220".to_string());
        }
        Self { slots }
    }
}

#[derive(Params)]
pub struct GlykonsHairParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    // TODO: Persistence for slots
    pub pattern_slots: Arc<RwLock<PatternSlots>>,

    #[id = "dummy"]
    pub dummy: FloatParam,
}

impl Default for GlykonsHairParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(1200, 800),
            pattern_slots: Arc::new(RwLock::new(PatternSlots::default())),
            dummy: FloatParam::new("Dummy", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}
