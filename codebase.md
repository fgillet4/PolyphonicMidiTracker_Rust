# .gitignore

```
/target
/Cargo.lock
**/*.rs.bk

```

# build.sh

```sh
#!/bin/bash

# Exit on error
set -e

# Set up color outputs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Guitar MIDI Tracker plugin...${NC}"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Rust is not installed. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

# Build the plugin in release mode
echo -e "${GREEN}Building plugin in release mode...${NC}"
cargo build --release

# Create a complete VST3 bundle for macOS
PLUGIN_NAME="GuitarMIDITracker"
VST3_BUNDLE_DIR="$HOME/Library/Audio/Plug-Ins/VST3/${PLUGIN_NAME}.vst3"
CONTENTS_DIR="${VST3_BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo -e "${GREEN}Creating VST3 bundle structure...${NC}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Create Info.plist
echo -e "${GREEN}Creating Info.plist...${NC}"
cat > "${CONTENTS_DIR}/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>${PLUGIN_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.your-name.${PLUGIN_NAME}</string>
    <key>CFBundleName</key>
    <string>${PLUGIN_NAME}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CSResourcesFileMapped</key>
    <true/>
</dict>
</plist>
EOF

# Create PkgInfo
echo -e "${GREEN}Creating PkgInfo...${NC}"
echo "BNDL????" > "${CONTENTS_DIR}/PkgInfo"

# Copy the compiled plugin
echo -e "${GREEN}Copying plugin to VST3 bundle...${NC}"
cp "target/release/libguitar_midi_tracker.dylib" "${MACOS_DIR}/${PLUGIN_NAME}"

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "Plugin is installed at: ${VST3_BUNDLE_DIR}"
echo -e "${YELLOW}Please restart Ableton Live and rescan plugins${NC}"
```

# Cargo.toml

```toml
[package]
name = "guitar_midi_tracker"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "Guitar to MIDI converter VST3 plugin"

[lib]
crate-type = ["cdylib"]

[dependencies]
nih_plug = { git = "https://github.com/robbert-vdh/nih-plug.git", features = ["vst3"] }
realfft = "3.3.0"  # For FFT processing
hound = "3.5.0"    # For WAV file handling during learning phase
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
atomic_float = "0.1"
num-complex = "0.4.3"  # For complex numbers in FFT

[workspace]
members = ["xtask"]

[profile.release]
lto = "thin"
strip = true

```

# data/.gitkeep

```

```

# src/fft_processor.rs

```rs
use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;
use std::f32::consts::PI;
use num_complex::Complex32;

pub struct FFTProcessor {
    fft_size: usize,
    sample_rate: f32,
    buffer: Vec<f32>,
    buffer_position: usize,
    window: Vec<f32>,
    fft: Option<Arc<dyn RealToComplex<f32>>>,
    spectrum: Vec<f32>,
}

impl FFTProcessor {
    pub fn new(fft_size: usize) -> Self {
        // Create a Hann window function
        let mut window = vec![0.0; fft_size];
        for i in 0..fft_size {
            window[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
        }
        
        Self {
            fft_size,
            sample_rate: 44100.0,  // Default, will be set in initialize
            buffer: vec![0.0; fft_size],
            buffer_position: 0,
            window,
            fft: None,
            spectrum: vec![0.0; fft_size / 2],
        }
    }
    
    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        
        // Initialize FFT
        let mut planner = RealFftPlanner::<f32>::new();
        self.fft = Some(planner.plan_fft_forward(self.fft_size));
        
        // Reset buffers
        self.reset();
    }
    
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.buffer_position = 0;
        self.spectrum.fill(0.0);
    }
    
    pub fn process_sample(&mut self, sample: f32) {
        // Add the sample to our buffer
        self.buffer[self.buffer_position] = sample;
        self.buffer_position = (self.buffer_position + 1) % self.fft_size;
    }
    
    pub fn is_frame_complete(&self) -> bool {
        // Check if we've collected a full buffer
        self.buffer_position == 0
    }
    
    pub fn compute_spectrum(&mut self) -> Vec<f32> {
        if let Some(fft) = &self.fft {
            // Apply window function
            let mut windowed_buffer = vec![0.0; self.fft_size];
            for i in 0..self.fft_size {
                let buffer_idx = (self.buffer_position + i) % self.fft_size;
                windowed_buffer[i] = self.buffer[buffer_idx] * self.window[i];
            }
            
            // Prepare for FFT
            let mut output_complex = vec![Complex32::new(0.0, 0.0); self.fft_size / 2 + 1];
            
            // Perform the FFT
            if let Ok(_) = fft.process(&mut windowed_buffer, &mut output_complex) {
                // Compute magnitudes
                for i in 0..self.fft_size / 2 {
                    let re = output_complex[i].re;
                    let im = output_complex[i].im;
                    let magnitude = (re * re + im * im).sqrt();
                    self.spectrum[i] = magnitude;
                }
            }
        }
        
        self.spectrum.clone()
    }
    
    pub fn get_frequency_for_bin(&self, bin: usize) -> f32 {
        bin as f32 * self.sample_rate / self.fft_size as f32
    }
}

```

# src/learning.rs

```rs
pub fn learn_note(note_detector: &mut crate::note_detection::NoteDetector, note: u8, spectrum: &[f32]) {
    // Add the current spectrum to our learned database for this note
    note_detector.add_learned_note(note, spectrum);
    
    println!("Learned note {}: {}", note, crate::utils::midi_note_to_name(note));
}

```

# src/lib.rs

```rs
use nih_plug::prelude::*;
use std::sync::Arc;
use atomic_float::AtomicF32;
use std::sync::atomic::{AtomicBool, Ordering};

mod learning;
mod tracking;
mod fft_processor;
mod spectral_analysis;
mod note_detection;
mod midi_output;
mod utils;

// Main plugin struct
pub struct GuitarMidiTracker {
    params: Arc<GuitarMidiTrackerParams>,
    sample_rate: f32,
    
    // FFT processing state
    fft_processor: fft_processor::FFTProcessor,
    
    // Learning mode state
    learning_mode: AtomicBool,
    current_learning_note: AtomicF32,
    
    // Tracking state
    note_detector: note_detection::NoteDetector,
    
    // Visualization data for UI
    fft_magnitude_buffer: Vec<f32>,
    detected_notes: Vec<u8>,
    
    // Buffer for processing
    sample_counter: usize,
}

#[derive(Params)]
struct GuitarMidiTrackerParams {
    #[id = "input_gain"]
    pub input_gain: FloatParam,
    
    #[id = "sensitivity"]
    pub sensitivity: FloatParam,
    
    #[id = "max_notes"]
    pub max_polyphony: IntParam,
    
    #[id = "learning_mode"]
    pub learning_mode: BoolParam,
    
    #[id = "learning_note"]
    pub learning_note: FloatParam,
    
    #[id = "save_learned_data"]
    pub save_learned_data: BoolParam,
    
    #[id = "load_learned_data"]
    pub load_learned_data: BoolParam,
}

impl Default for GuitarMidiTrackerParams {
    fn default() -> Self {
        Self {
            input_gain: FloatParam::new(
                "Input Gain", 
                0.0, 
                FloatRange::Linear { min: -12.0, max: 12.0 }
            )
            .with_unit(" dB")
            .with_smoother(SmoothingStyle::Logarithmic(50.0)),
            
            sensitivity: FloatParam::new(
                "Sensitivity",
                0.5,
                FloatRange::Linear { min: 0.1, max: 1.0 }
            ),
            
            max_polyphony: IntParam::new(
                "Max Polyphony",
                6,
                IntRange::Linear { min: 1, max: 12 }
            ),
            
            learning_mode: BoolParam::new("Learning Mode", false),
            
            learning_note: FloatParam::new(
                "Learning Note",
                60.0, // Middle C
                FloatRange::Linear { min: 40.0, max: 90.0 }
            )
            .with_value_to_string(Arc::new(|v| format!("{} ({})", v, utils::midi_note_to_name(v as u8))))
            .with_string_to_value(Arc::new(|s| s.parse::<f32>().ok())),
            
            save_learned_data: BoolParam::new("Save Learned Data", false)
                .with_callback(Arc::new(move |value| {
                    if value {
                        // Trigger save functionality
                        println!("Saving learned data...");
                    }
                })),
                
            load_learned_data: BoolParam::new("Load Learned Data", false)
                .with_callback(Arc::new(move |value| {
                    if value {
                        // Trigger load functionality
                        println!("Loading learned data...");
                    }
                })),
        }
    }
}

impl Default for GuitarMidiTracker {
    fn default() -> Self {
        Self {
            params: Arc::new(GuitarMidiTrackerParams::default()),
            sample_rate: 44100.0,
            fft_processor: fft_processor::FFTProcessor::new(4096),
            learning_mode: AtomicBool::new(false),
            current_learning_note: AtomicF32::new(60.0), // Middle C
            note_detector: note_detection::NoteDetector::new(),
            fft_magnitude_buffer: Vec::new(),
            detected_notes: Vec::new(),
            sample_counter: 0,
        }
    }
}

impl Plugin for GuitarMidiTracker {
    const NAME: &'static str = "Guitar MIDI Tracker";
    const VENDOR: &'static str = "FrancisBrain";
    const URL: &'static str = "https://your-website.com";
    const EMAIL: &'static str = "fgillet4@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.fft_processor.initialize(buffer_config.sample_rate);
        self.note_detector.initialize(buffer_config.sample_rate);
        self.sample_counter = 0;
        
        true
    }

    fn reset(&mut self) {
        self.fft_processor.reset();
        self.note_detector.reset();
        self.sample_counter = 0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Check if we should be in learning mode
        let learning_mode = self.params.learning_mode.value();
        self.learning_mode.store(learning_mode, Ordering::Relaxed);
        let learning_note_midi = self.params.learning_note.value() as u8;
        
        // Process the buffer sample by sample
        for mut channels in buffer.iter_samples() {
            // Calculate the average input from all channels
            let mut input_sample = 0.0;
            let mut sample_count = 0;
            
            // Iterate through all channels at this sample position
            for channel_sample in channels.iter_mut() {
                input_sample += *channel_sample;
                sample_count += 1;
            }
            
            if sample_count > 0 {
                input_sample /= sample_count as f32;
                
                // Apply input gain
                let gain = self.params.input_gain.smoothed.next();
                input_sample *= utils::db_to_gain(gain);
                
                // Process the sample through FFT
                self.fft_processor.process_sample(input_sample);
                
                // Write the processed sample to all output channels
                for channel_sample in channels.iter_mut() {
                    *channel_sample = input_sample;
                }
                
                // Track samples for FFT analysis
                self.sample_counter += 1;
                if self.sample_counter >= 4096 {
                    self.sample_counter = 0;
                    
                    // When we have enough samples, compute the spectrum
                    let spectrum = self.fft_processor.compute_spectrum();
                    self.fft_magnitude_buffer = spectrum.clone();
                    
                    if learning_mode {
                        // Learning mode - learn the current note
                        learning::learn_note(&mut self.note_detector, learning_note_midi, &spectrum);
                    } else {
                        // Tracking mode - detect notes
                        let max_notes = self.params.max_polyphony.value() as usize;
                        let sensitivity = self.params.sensitivity.value();
                        let detected_notes = self.note_detector.detect_notes(&spectrum, max_notes, sensitivity);
                        
                        // Output MIDI notes
                        midi_output::output_midi_notes(context, &detected_notes, &self.detected_notes);
                        
                        // Update stored note state
                        self.detected_notes = detected_notes;
                    }
                }
            }
        }
        
        ProcessStatus::Normal
    }
}

impl ClapPlugin for GuitarMidiTracker {
    const CLAP_ID: &'static str = "com.your-name.guitar-midi-tracker";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Convert guitar audio to MIDI");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Utility,
        ClapFeature::Analyzer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for GuitarMidiTracker {
    const VST3_CLASS_ID: [u8; 16] = *b"GuitarMIDITrackr"; // 16 bytes exactly
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Tools,
        Vst3SubCategory::Analyzer,
    ];
}

// Export the plugins using the proper macro syntax
nih_export_clap!(GuitarMidiTracker);
nih_export_vst3!(GuitarMidiTracker);

```

# src/midi_output.rs

```rs
use nih_plug::prelude::*;

pub fn output_midi_notes<P: Plugin>(
    context: &mut impl ProcessContext<P>,
    current_notes: &[u8],
    previous_notes: &[u8],
) {
    // Find notes that need to be turned off (in previous but not in current)
    for &note in previous_notes {
        if !current_notes.contains(&note) {
            let event = NoteEvent::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note,
                velocity: 0.0,
            };
            context.send_event(event);
        }
    }
    
    // Find notes that need to be turned on (in current but not in previous)
    for &note in current_notes {
        if !previous_notes.contains(&note) {
            let event = NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note,
                velocity: 0.8,  // Fixed velocity for now
            };
            context.send_event(event);
        }
    }
}

```

# src/note_detection.rs

```rs
use std::collections::HashMap;
use std::sync::RwLock;

pub struct NoteDetector {
    sample_rate: f32,
    learned_notes: RwLock<HashMap<u8, Vec<f32>>>,
    note_thresholds: Vec<f32>,
}

impl NoteDetector {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            learned_notes: RwLock::new(HashMap::new()),
            note_thresholds: vec![0.0; 128],
        }
    }
    
    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
    
    pub fn reset(&mut self) {
        // Nothing to reset in this implementation
    }
    
    pub fn add_learned_note(&self, note: u8, spectrum: &[f32]) {
        let mut notes = self.learned_notes.write().unwrap();
        notes.insert(note, spectrum.to_vec());
    }
    
    pub fn detect_notes(&self, spectrum: &[f32], max_notes: usize, sensitivity: f32) -> Vec<u8> {
        let notes = self.learned_notes.read().unwrap();
        if notes.is_empty() {
            return Vec::new();
        }
        
        // Calculate similarity score for each learned note
        let mut note_scores: Vec<(u8, f32)> = Vec::new();
        
        for (&note, &ref learned_spectrum) in notes.iter() {
            let similarity = self.calculate_similarity(spectrum, learned_spectrum);
            if similarity > sensitivity * 0.5 {  // Threshold based on sensitivity
                note_scores.push((note, similarity));
            }
        }
        
        // Sort by similarity score in descending order
        note_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Take the top max_notes
        note_scores.truncate(max_notes);
        
        // Return just the notes
        note_scores.iter().map(|(note, _)| *note).collect()
    }
    
    fn calculate_similarity(&self, spectrum: &[f32], learned_spectrum: &[f32]) -> f32 {
        // Simple cosine similarity implementation
        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        
        let len = spectrum.len().min(learned_spectrum.len());
        
        for i in 0..len {
            dot_product += spectrum[i] * learned_spectrum[i];
            norm_a += spectrum[i] * spectrum[i];
            norm_b += learned_spectrum[i] * learned_spectrum[i];
        }
        
        if norm_a.sqrt() * norm_b.sqrt() > 0.0 {
            dot_product / (norm_a.sqrt() * norm_b.sqrt())
        } else {
            0.0
        }
    }
    
    pub fn save_learned_data(&self, path: &str) -> Result<(), std::io::Error> {
        let notes = self.learned_notes.read().unwrap();
        let json = serde_json::to_string(&*notes)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    pub fn load_learned_data(&self, path: &str) -> Result<(), std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let loaded_notes: HashMap<u8, Vec<f32>> = serde_json::from_str(&json)?;
        
        let mut notes = self.learned_notes.write().unwrap();
        *notes = loaded_notes;
        
        Ok(())
    }
}

```

# src/spectral_analysis.rs

```rs
// This module would contain more advanced spectral analysis techniques
// For now, it's a placeholder as the basic spectral analysis is in fft_processor.rs

```

# src/tracking.rs

```rs
// This module would contain any tracking-specific logic
// For now, it's a placeholder as the main tracking code is in lib.rs

```

# src/utils.rs

```rs
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

pub fn midi_note_to_freq(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

pub fn midi_note_to_name(note: u8) -> String {
    const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = note / 12 - 1;
    let note_name = NOTE_NAMES[(note % 12) as usize];
    format!("{}{}", note_name, octave)
}

```

# xtask/Cargo.toml

```toml
[package]
name = "xtask"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
nih_plug_xtask = { git = "https://github.com/robbert-vdh/nih-plug.git" }
```

# xtask/src/main.rs

```rs
fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()
}

```

