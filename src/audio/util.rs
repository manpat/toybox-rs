use crate::prelude::*;


/// Convert a midi note to a frequency.
/// Integer parts correspond directly midi note values - where note 12 is C0 (16.35Hz) and
/// note 69 is A4 (440Hz). The fractional part gives sub-semitone shifts - where 1 cent is 0.01 note.
pub fn midi_note_to_frequency(midi_note: f32) -> f32 {
	440.0 * ((midi_note - 69.0)/12.0).exp2()
}

/// Convert a frequency to a midi note.
/// See: midi_note_to_frequency.
pub fn frequency_to_midi_note(frequency: f32) -> f32 {
	(frequency/440.0).log2() * 12.0 + 69.0
}



// TODO(pat.m): note classification
// TODO(pat.m): chord/scale construction


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PitchClass {
	C, Cs,
	D, Ds,
	E,
	F, Fs,
	G, Gs,
	A, As,
	B,
}

impl PitchClass {
	pub fn from_midi(midi_note: i32) -> PitchClass {
		match midi_note.rem_euclid(12) {
			0 => PitchClass::C,
			1 => PitchClass::Cs,
			2 => PitchClass::D,
			3 => PitchClass::Ds,
			4 => PitchClass::E,
			5 => PitchClass::F,
			6 => PitchClass::Fs,
			7 => PitchClass::G,
			8 => PitchClass::Gs,
			9 => PitchClass::A,
			10 => PitchClass::As,
			11 => PitchClass::B,
			_ => unreachable!()
		}
	}

	pub fn to_midi(&self, octave: i32) -> i32 {
		let note = match self {
			PitchClass::C => 0,
			PitchClass::Cs => 1,
			PitchClass::D => 2,
			PitchClass::Ds => 3,
			PitchClass::E => 4,
			PitchClass::F => 5,
			PitchClass::Fs => 6,
			PitchClass::G => 7,
			PitchClass::Gs => 8,
			PitchClass::A => 9,
			PitchClass::As => 10,
			PitchClass::B => 11,
		};

		(octave+1)*12 + note
	}
}

use std::fmt;

impl fmt::Display for PitchClass {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			PitchClass::C => "C".fmt(f),
			PitchClass::Cs => "C#".fmt(f),
			PitchClass::D => "D".fmt(f),
			PitchClass::Ds => "D#".fmt(f),
			PitchClass::E => "E".fmt(f),
			PitchClass::F => "F".fmt(f),
			PitchClass::Fs => "F#".fmt(f),
			PitchClass::G => "G".fmt(f),
			PitchClass::Gs => "G#".fmt(f),
			PitchClass::A => "A".fmt(f),
			PitchClass::As => "A#".fmt(f),
			PitchClass::B => "B".fmt(f),
		}
	}
}


#[derive(Copy, Clone, Debug)]
pub struct Pitch {
	pub pitch_class: PitchClass,
	pub octave: i32,
	pub cents: f32, // (-1, 1)
}

impl Pitch {
	pub fn from_midi(midi_note: f32) -> Pitch {
		let cents = midi_note.fract();
		let midi_note = midi_note.trunc() as i32;
		let octave = midi_note/12 - 1;
		let pitch_class = PitchClass::from_midi(midi_note);

		Pitch {
			pitch_class,
			octave,
			cents,
		}
	}

	pub fn from_frequency(frequency: f32) -> Pitch {
		let midi_note = frequency_to_midi_note(frequency);
		Pitch::from_midi(midi_note)
	}

	pub fn to_midi(&self) -> f32 {
		self.pitch_class.to_midi(self.octave) as f32 + self.cents
	}

	pub fn to_frequency(&self) -> f32 {
		midi_note_to_frequency(self.to_midi())
	}
}