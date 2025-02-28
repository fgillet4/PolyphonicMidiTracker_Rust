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
    const NAME: &'static str = "GuitarMIDITracker";
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

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
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
        nih_log!("GuitarMIDITracker: Initializing with sample rate: {}", buffer_config.sample_rate);
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
        nih_log!("GuitarMIDITracker: Processing audio buffer");
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
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Analyzer,
    ];
}

// Export the plugins using the proper macro syntax
nih_export_clap!(GuitarMidiTracker);
nih_export_vst3!(GuitarMidiTracker);
