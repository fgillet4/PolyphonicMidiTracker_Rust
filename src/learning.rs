pub fn learn_note(note_detector: &mut crate::note_detection::NoteDetector, note: u8, spectrum: &[f32]) {
    // Add the current spectrum to our learned database for this note
    note_detector.add_learned_note(note, spectrum);
    
    println!("Learned note {}: {}", note, crate::utils::midi_note_to_name(note));
}
