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
