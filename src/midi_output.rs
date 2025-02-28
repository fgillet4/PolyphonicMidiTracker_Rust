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
