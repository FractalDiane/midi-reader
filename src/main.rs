// MIDI File parser
// I'll use it for... something
// Written by Diane Sparks

use std::{env, io::SeekFrom};
use std::process;
use std::io::Seek;
use std::fs::File;
use std::collections::hash_map::HashMap;
use std::fmt;
use std::cmp::PartialEq;
use byteorder::{BigEndian, ReadBytesExt};

extern crate num;
#[macro_use]
extern crate num_derive;
use num::{FromPrimitive, cast};

#[cfg(feature="midi_debug")]
macro_rules! debug_print {
	() => { println!() };
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(feature="midi_debug"))]
macro_rules! debug_print {
	() => { };
    ($($arg:tt)*) => { };
}

#[derive(PartialEq)]
enum EventClass {
	Channel,
	SysEx,
	Meta,
}

#[derive(FromPrimitive, PartialEq)]
enum ChannelEvent {
	NoteOff = 0x8,
	NoteOn = 0x9,
	NoteAftertouch = 0xa,
	Controller = 0xb,
	ProgramChange = 0xc,
	ChannelAftertouch = 0xd,
	PitchBend = 0xe,
}

#[derive(FromPrimitive, PartialEq)]
enum MetaEvent {
	SequenceNumber = 0x00,
	Text = 0x01,
	Copyright = 0x02,
	TrackName = 0x03,
	InstrumentName = 0x04,
	Lyric = 0x05,
	Marker = 0x06,
	CuePoint = 0x07,
	ChannelPrefix = 0x20,
	MidiBus = 0x21,
	EndOfTrack = 0x2f,
	SetTempo = 0x51,
	SMPTEOffset = 0x54,
	TimeSignature = 0x58,
	KeySignature = 0x59,
	SequencerMetaEvent = 0x7f,
}


struct Event {
	class: EventClass,
	delta: u32,
	status: u8,
	params: Vec<u8>,
}

#[derive(PartialEq, Eq, Hash/*, Clone, Copy*/)]
struct MidiNote {
	key: u8,
	velocity: u8,
	delta: u32,
	position: u32,
}


struct Note {
	key: u8,
	velocity: u8,
	position: u32,
	duration: u32,
}


impl fmt::Display for Note {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "[Key {}, Velocity {}] {} -> {}", self.key, self.velocity, self.position, self.position + self.duration)
	}
}


fn high4(value: u8) -> u8 {
	value >> 4
}


fn low4(value: u8) -> u8 {
	value & 0xf
}


fn read_string(file: &mut File, len: usize) -> String {
	let mut arr =  vec!['\0'; len];
	for i in 0..len {
		arr[i] = file.read_u8().unwrap() as char;
	}

	arr.iter().collect()
}


fn read_vlq(file: &mut File, byte_count: &mut u32) -> u32 {
	let mut result: u32 = 0;
	let mut buffer: u32 = 0b10000000;
	let mut i: u8 = 0;
	while buffer & 0x80 != 0 {
		buffer = file.read_u8().unwrap() as u32;
		result |= (buffer & 0x7f) << (7 * i);
		*byte_count += 1;
		i += 1;
	}

	result
}


fn read_channel_event(delta: u32, status: u8, file: &mut File, byte_count: &mut u32) -> Event {
	let typ: ChannelEvent = FromPrimitive::from_u8(high4(status)).unwrap();
	let mut params = Vec::<u8>::new();
	match typ {
		ChannelEvent::NoteOff => {
			let note = file.read_u8().unwrap();
			let velocity = file.read_u8().unwrap();
			debug_print!("Note Off\nNote: {}\nVelocity: {}\n", note, velocity);
			*byte_count += 2;

			params.push(note);
			params.push(velocity);
		},

		ChannelEvent::NoteOn => {
			let note = file.read_u8().unwrap();
			let velocity = file.read_u8().unwrap();
			debug_print!("Note On\nNote: {}\nVelocity: {}\n", note, velocity);
			*byte_count += 2;

			params.push(note);
			params.push(velocity);
		},

		ChannelEvent::NoteAftertouch => {
			let note = file.read_u8().unwrap();
			let pressure = file.read_u8().unwrap();
			debug_print!("Note Aftertouch\nNote: {}\nPressure: {}\n", note, pressure);
			*byte_count += 2;

			params.push(note);
			params.push(pressure);
		},

		ChannelEvent::Controller => {
			let controller = file.read_u8().unwrap();
			let value = file.read_u8().unwrap();
			debug_print!("Controller change\nController: {}\nValue: {}\n", controller, value);
			*byte_count += 2;

			params.push(controller);
			params.push(value);
		},

		ChannelEvent::ProgramChange => {
			let number = file.read_u8().unwrap();
			debug_print!("Program change\nProgram: {}\n", number);
			*byte_count += 1;

			params.push(number);
		},

		ChannelEvent::ChannelAftertouch => {
			let pressure = file.read_u8().unwrap();
			debug_print!("Channel aftertouch\nPressure: {}\n", pressure);
			*byte_count += 1;

			params.push(pressure);
		},

		ChannelEvent::PitchBend => {
			let low = file.read_u8().unwrap() as u16;
			let high = file.read_u8().unwrap() as u16;
			debug_print!("Pitch bend\nValue: {}\n", (high << 8) | low);
			*byte_count += 2;

			params.push(low as u8);
			params.push(high as u8);
		},
	}

	Event{class: EventClass::Channel, delta, status, params}
}


fn read_meta_event(delta: u32, event_type: u8, file: &mut File, byte_count: &mut u32) -> Event {
	//let typ: MetaEvent = FromPrimitive::from_u8(event_type).unwrap();
	let length = file.read_u8().unwrap() as u32;
	*byte_count += 1;
	let mut params = Vec::<u8>::new();
	for _i in 0..length {
		params.push(file.read_u8().unwrap());
		*byte_count += 1;
	}

	debug_print!("Type 0x{:x}", event_type);
	debug_print!("Length: {}", length);
	debug_print!("Parameters: {:?}\n", params);

	Event{class: EventClass::Meta, delta, status: event_type, params}
}


fn read_event(file: &mut File, byte_count: &mut u32, prev_status: &mut u8) -> Event {
	let delta = read_vlq(file, byte_count);
	debug_print!("Delta {}", delta);

	let status = file.read_u8().unwrap();
	*byte_count += 1;
	match status {
		0xff => {
			debug_print!("Meta event");
			let typ = file.read_u8().unwrap();
			*byte_count += 1;
			read_meta_event(delta, typ, file, byte_count)
		},
		0xf0 => {
			debug_print!("SysEx event");
			Event{class: EventClass::SysEx, delta: 0, status: 0x0, params: vec![]}
		},
		0xf7 => {
			debug_print!("SysEx event (end)");
			Event{class: EventClass::SysEx, delta: 0, status: 0x0, params: vec![]}
		},
		_ => {
			debug_print!("Normal event");

			let typ: u8;
			let channel: u8;
			// Running status:
			// If the high bit isn't set, we're reusing the same
			// status byte from the last event.
			if status < 0x80 {
				typ = high4(*prev_status);
				channel = low4(*prev_status);
				file.seek(SeekFrom::Current(-1)).unwrap();
				*byte_count -= 1;
			}
			else {
				typ = high4(status);
				channel = low4(status);
				*prev_status = status;
			}

			debug_print!("Type: 0x{:x}\nChannel: {}", typ, channel);

			read_channel_event(delta, *prev_status, file, byte_count)
		},
	}
}


fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		process::exit(1);
	}

	let mut file = File::open(&args[1]).expect(format!("Failed to open file {}", &args[1]).as_str());

	let chunk_id = read_string(&mut file, 4);
	if chunk_id != "MThd" {
		eprintln!("Error: Not a midi file");
		process::exit(1);
	}

	let _chunk_size = file.read_u32::<BigEndian>().unwrap();
	let format = file.read_u16::<BigEndian>().unwrap();
	let tracks = file.read_u16::<BigEndian>().unwrap();
	let time = file.read_u16::<BigEndian>().unwrap();
	
	println!("File type: {}\nTracks: {}", format, tracks);

	if time & 0x8000 == 0 {
		println!("Time division: {} ticks/beat\n", time & 0x7fff);
	}
	else {
		println!("Time division: {} frames/second\n", time & 0x7fff);
	}

	let mut notes = Vec::<Vec::<Note>>::new();
	
	for track in 0..tracks {
		//println!("Track {}", track);

		notes.push(Vec::<Note>::new());

		let this_chunk_id = read_string(&mut file, 4);
		if this_chunk_id != "MTrk" {
			eprintln!("Error: Invalid track format");
			process::exit(1);
		}

		let this_chunk_size = file.read_u32::<BigEndian>().unwrap();
		//println!("{} bytes\n", this_chunk_size);

		let mut byte_count = 0;
		let mut current_status: u8 = 0x0;
		let mut total_delta: u32 = 0;

		let mut ongoing_notes = HashMap::<u8, MidiNote>::new();

		while byte_count < this_chunk_size {
			let event = read_event(&mut file, &mut byte_count, &mut current_status);
			total_delta += event.delta;
			if event.class == EventClass::Channel {
				let typ: ChannelEvent = FromPrimitive::from_u8(high4(event.status)).unwrap();
				if typ == ChannelEvent::NoteOff || (typ == ChannelEvent::NoteOn && event.params[1] == 0) {
					if ongoing_notes.contains_key(&event.params[0]) {
						let note = ongoing_notes.remove(&event.params[0]).unwrap();
						notes[track as usize].push(Note{key: note.key, velocity: note.velocity, position: note.position, duration: total_delta - note.position});
					}
				}
				else if typ == ChannelEvent::NoteOn {
					let new_note = MidiNote{key: event.params[0], velocity: event.params[1], delta: event.delta, position: total_delta};
					ongoing_notes.insert(new_note.key, new_note);
				}
			}
		}
	}

	for i in 0..notes.len() {
		for note in &notes[i] {
			println!("Track {}: {}", i, *note);
		}
	}
}
