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
mod ui;

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
    
    #[persist = "editor_state"]
    editor_state: Arc<parking_lot::RwLock<ui::EditorState>>,
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
            .with_value_to_string(formatters::v2s_f32_midi_note())
            .with_string_to_value(formatters::s2v_f32_midi_note()),
            
            save_learned_data: BoolParam::new("Save Learned Data", false)
                .with_callback({
                    let learning_mode = Arc::new(AtomicBool::new(false));
                    let learning_mode_clone = learning_mode.clone();
                    
                    Arc::new(move |value| {
                        if value {
                            // Trigger save functionality
                            println!("Saving learned data...");
                            // Reset parameter after handling
                            learning_mode_clone.store(false, Ordering::Relaxed);
                        }
                    })
                }),
                
            load_learned_data: BoolParam::new("Load Learned Data", false)
                .with_callback({
                    let load_trigger = Arc::new(AtomicBool::new(false));
                    let load_trigger_clone = load_trigger.clone();
                    
                    Arc::new(move |value| {
                        if value {
                            // Trigger load functionality
                            println!("Loading learned data...");
                            // Reset parameter after handling
                            load_trigger_clone.store(false, Ordering::Relaxed);
                        }
                    })
                }),
                
            editor_state: Arc::new(parking_lot::RwLock::new(ui::EditorState::default())),
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
        }
    }
}

impl Plugin for GuitarMidiTracker {
    const NAME: &'static str = "Guitar MIDI Tracker";
    const VENDOR: &'static str = "Your Name";
    const URL: &'static str = "https://your-website.com";
    const EMAIL: &'static str = "your.email@example.com";
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

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create_editor(self.params.clone())
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
        
        true
    }

    fn reset(&mut self) {
        self.fft_processor.reset();
        self.note_detector.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Process audio buffer
        let num_samples = buffer.samples();
        let num_channels = buffer.channels();
        
        // Check if we should be in learning mode
        let learning_mode = self.params.learning_mode.value();
        self.learning_mode.store(learning_mode, Ordering::Relaxed);
        
        if learning_mode {
            // Learning mode - analyze individual notes
            let learning_note_midi = self.params.learning_note.value() as u8;
            self.current_learning_note.store(learning_note_midi as f32, Ordering::Relaxed);
            
            // Process audio for learning
            for i in 0..num_samples {
                // Get mono input (average channels if stereo)
                let mut input_sample = 0.0;
                for channel in 0..num_channels.min(2) {
                    input_sample += buffer[channel][i];
                }
                input_sample /= num_channels.min(2) as f32;
                
                // Apply input gain
                let gain = self.params.input_gain.smoothed.next();
                input_sample *= utils::db_to_gain(gain);
                
                // Process the sample for learning
                self.fft_processor.process_sample(input_sample);
                
                // For passthrough monitoring, copy input to output
                for channel in 0..buffer.channels() {
                    buffer[channel][i] = input_sample;
                }
            }
            
            // Check if we have a complete FFT frame
            if self.fft_processor.is_frame_complete() {
                let spectrum = self.fft_processor.compute_spectrum();
                
                // Update FFT visualization buffer
                self.fft_magnitude_buffer = spectrum.clone();
                
                // Learn the current note
                let note_midi = self.current_learning_note.load(Ordering::Relaxed) as u8;
                learning::learn_note(&mut self.note_detector, note_midi, &spectrum);
            }
        } else {
            // Tracking mode - detect notes from polyphonic input
            for i in 0..num_samples {
                // Get mono input (average channels if stereo)
                let mut input_sample = 0.0;
                for channel in 0..num_channels.min(2) {
                    input_sample += buffer[channel][i];
                }
                input_sample /= num_channels.min(2) as f32;
                
                // Apply input gain
                let gain = self.params.input_gain.smoothed.next();
                input_sample *= utils::db_to_gain(gain);
                
                // Process the sample for note detection
                self.fft_processor.process_sample(input_sample);
                
                // For passthrough monitoring, copy input to output
                for channel in 0..buffer.channels() {
                    buffer[channel][i] = input_sample;
                }
            }
            
            // Check if we have a complete FFT frame
            if self.fft_processor.is_frame_complete() {
                let spectrum = self.fft_processor.compute_spectrum();
                
                // Update FFT visualization buffer
                self.fft_magnitude_buffer = spectrum.clone();
                
                // Detect notes from the spectrum
                let max_notes = self.params.max_polyphony.value() as usize;
                let sensitivity = self.params.sensitivity.value();
                let detected_notes = self.note_detector.detect_notes(&spectrum, max_notes, sensitivity);
                
                // Output MIDI notes
                midi_output::output_midi_notes(context, &detected_notes, &self.detected_notes);
                
                // Update our stored note state
                self.detected_notes = detected_notes;
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
    const VST3_CLASS_ID: [u8; 16] = *b"GuitarMIDITracker";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Tools,
        Vst3SubCategory::Analyzer,
    ];
}

nih_export_clap!(GuitarMidiTracker);
nih_export_vst3!(GuitarMidiTracker);
nih_export_standalone!(GuitarMidiTracker);