#![windows_subsystem = "windows"]

use glykons_hair::GlykonsHair;
use nih_plug::prelude::*;

fn main() {
    nih_export_standalone::<GlykonsHair>();
}
