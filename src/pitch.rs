use crate::music::{Note, SOUNDING_MAX_MIDI, SOUNDING_MIN_MIDI};
use pitch_detection::detector::PitchDetector as DetectorTrait;
use pitch_detection::detector::yin::YINDetector;

pub const WINDOW_SIZE: usize = 4096;
pub const POWER_THRESHOLD: f32 = 1.0;
pub const CLARITY_THRESHOLD: f32 = 0.7;
pub const CENTS_TOLERANCE: f32 = 50.0;

#[derive(Debug, Clone, Copy)]
pub struct PitchEvent {
    pub frequency: f32,
    pub clarity: f32,
    pub note: Note,
    pub cents: f32,
}

pub trait PitchDetector {
    fn detect(&mut self, samples: &[f32], sample_rate: u32) -> Option<PitchEvent>;
}

pub struct YinDetector {
    inner: YINDetector<f32>,
    size: usize,
}

impl YinDetector {
    pub fn new(size: usize) -> Self {
        Self {
            inner: YINDetector::new(size, size / 2),
            size,
        }
    }
}

impl PitchDetector for YinDetector {
    fn detect(&mut self, samples: &[f32], sample_rate: u32) -> Option<PitchEvent> {
        if samples.len() < self.size {
            return None;
        }
        let window = &samples[samples.len() - self.size..];
        let pitch = self.inner.get_pitch(
            window,
            sample_rate as usize,
            POWER_THRESHOLD,
            CLARITY_THRESHOLD,
        )?;
        freq_to_event(pitch.frequency, pitch.clarity)
    }
}

pub fn freq_to_event(frequency: f32, clarity: f32) -> Option<PitchEvent> {
    if !frequency.is_finite() || frequency <= 0.0 {
        return None;
    }
    let semis_from_a4 = 12.0 * (frequency / 440.0).log2();
    let rounded = semis_from_a4.round();
    let midi = (rounded as i32) + 69;
    let cents = (semis_from_a4 - rounded) * 100.0;

    if cents.abs() > CENTS_TOLERANCE {
        return None;
    }
    if midi < SOUNDING_MIN_MIDI || midi > SOUNDING_MAX_MIDI {
        return None;
    }

    Some(PitchEvent {
        frequency,
        clarity,
        note: Note::from_midi(midi),
        cents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a2_round_trips() {
        let evt = freq_to_event(110.0, 1.0).unwrap();
        assert_eq!(evt.note.midi, 45);
        assert!((evt.cents).abs() < 0.001);
    }

    #[test]
    fn e1_round_trips() {
        let evt = freq_to_event(41.203, 1.0).unwrap();
        assert_eq!(evt.note.midi, 28);
    }

    #[test]
    fn out_of_range_rejected() {
        assert!(freq_to_event(20.0, 1.0).is_none());
        assert!(freq_to_event(2000.0, 1.0).is_none());
    }

    #[test]
    fn detunes_under_50_cents_accepted() {
        let evt = freq_to_event(111.0, 1.0).unwrap();
        assert_eq!(evt.note.midi, 45);
        assert!(evt.cents > 0.0 && evt.cents < 50.0);
    }
}
