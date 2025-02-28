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
