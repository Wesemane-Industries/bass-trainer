use serde::{Deserialize, Serialize};
use std::fmt;

pub const STRING_OPEN_MIDI: [i32; 4] = [28, 33, 38, 43];
pub const STRING_LABELS: [&str; 4] = ["E", "A", "D", "G"];
pub const MAX_FRET: i32 = 20;

pub const SOUNDING_MIN_MIDI: i32 = 28;
pub const SOUNDING_MAX_MIDI: i32 = STRING_OPEN_MIDI[3] + MAX_FRET;

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Note {
    pub midi: i32,
}

impl Note {
    pub fn from_midi(midi: i32) -> Self {
        Self { midi }
    }

    pub fn name(self) -> &'static str {
        NOTE_NAMES[self.midi.rem_euclid(12) as usize]
    }

    pub fn octave(self) -> i32 {
        self.midi.div_euclid(12) - 1
    }

    pub fn pitch_class(self) -> i32 {
        self.midi.rem_euclid(12)
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.name(), self.octave())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Chromatic,
    Major,
    Minor,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::Chromatic => "chromatic",
            Mode::Major => "major",
            Mode::Minor => "minor",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub root_pc: i32,
    pub mode: Mode,
}

impl Key {
    pub fn new(root_pc: i32, mode: Mode) -> Self {
        Self {
            root_pc: root_pc.rem_euclid(12),
            mode,
        }
    }

    pub fn root_name(self) -> &'static str {
        NOTE_NAMES[self.root_pc as usize]
    }

    pub fn scale_intervals(self) -> &'static [i32] {
        match self.mode {
            Mode::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Mode::Major => &[0, 2, 4, 5, 7, 9, 11],
            Mode::Minor => &[0, 2, 3, 5, 7, 8, 10],
        }
    }

    pub fn contains(self, note: Note) -> bool {
        if matches!(self.mode, Mode::Chromatic) {
            return true;
        }
        let rel = (note.pitch_class() - self.root_pc).rem_euclid(12);
        self.scale_intervals().contains(&rel)
    }

    pub fn diatonic_notes_in_range(self, min_midi: i32, max_midi: i32) -> Vec<Note> {
        (min_midi..=max_midi)
            .map(Note::from_midi)
            .filter(|n| self.contains(*n))
            .collect()
    }

    pub fn all() -> Vec<Key> {
        let mut out = Vec::with_capacity(25);
        out.push(Key::new(0, Mode::Chromatic));
        for pc in 0..12 {
            out.push(Key::new(pc, Mode::Major));
        }
        for pc in 0..12 {
            out.push(Key::new(pc, Mode::Minor));
        }
        out
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            Mode::Chromatic => f.write_str("All notes (chromatic)"),
            _ => write!(f, "{} {}", self.root_name(), self.mode),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FretPosition {
    pub string_index: usize,
    pub fret: i32,
}

pub fn fret_positions(note: Note) -> Vec<FretPosition> {
    let mut out = Vec::new();
    for (idx, &open) in STRING_OPEN_MIDI.iter().enumerate() {
        let fret = note.midi - open;
        if (0..=MAX_FRET).contains(&fret) {
            out.push(FretPosition {
                string_index: idx,
                fret,
            });
        }
    }
    out
}

pub fn drill_pool(key: Key) -> Vec<Note> {
    key.diatonic_notes_in_range(SOUNDING_MIN_MIDI, SOUNDING_MAX_MIDI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_name_and_octave() {
        let e1 = Note::from_midi(28);
        assert_eq!(e1.name(), "E");
        assert_eq!(e1.octave(), 1);
        assert_eq!(format!("{}", e1), "E1");

        let a4 = Note::from_midi(69);
        assert_eq!(a4.name(), "A");
        assert_eq!(a4.octave(), 4);
    }

    #[test]
    fn open_strings_have_correct_midi() {
        assert_eq!(Note::from_midi(STRING_OPEN_MIDI[0]).to_string(), "E1");
        assert_eq!(Note::from_midi(STRING_OPEN_MIDI[1]).to_string(), "A1");
        assert_eq!(Note::from_midi(STRING_OPEN_MIDI[2]).to_string(), "D2");
        assert_eq!(Note::from_midi(STRING_OPEN_MIDI[3]).to_string(), "G2");
    }

    #[test]
    fn c_major_contains_c_e_g_but_not_csharp() {
        let k = Key::new(0, Mode::Major);
        assert!(k.contains(Note::from_midi(60)));
        assert!(k.contains(Note::from_midi(64)));
        assert!(k.contains(Note::from_midi(67)));
        assert!(!k.contains(Note::from_midi(61)));
    }

    #[test]
    fn a_minor_contains_natural_minor_notes() {
        let k = Key::new(9, Mode::Minor);
        for &pc in &[9, 11, 0, 2, 4, 5, 7] {
            assert!(k.contains(Note::from_midi(60 + pc)));
        }
        assert!(!k.contains(Note::from_midi(60 + 8)));
    }

    #[test]
    fn fret_positions_for_a2() {
        let a2 = Note::from_midi(45);
        let positions = fret_positions(a2);
        assert!(positions.contains(&FretPosition { string_index: 0, fret: 17 }));
        assert!(positions.contains(&FretPosition { string_index: 1, fret: 12 }));
        assert!(positions.contains(&FretPosition { string_index: 2, fret: 7 }));
        assert!(positions.contains(&FretPosition { string_index: 3, fret: 2 }));
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn drill_pool_in_range() {
        let pool = drill_pool(Key::new(7, Mode::Major));
        for n in &pool {
            assert!(n.midi >= SOUNDING_MIN_MIDI);
            assert!(n.midi <= SOUNDING_MAX_MIDI);
        }
        assert!(!pool.is_empty());
    }
}
