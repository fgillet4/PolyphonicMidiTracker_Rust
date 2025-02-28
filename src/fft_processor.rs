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
            
            // Perform FFT
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
