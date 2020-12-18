// MIDI File parser
// I'll use it for... something
// Written by Diane Sparks

use std::{env, io::SeekFrom};
use std::process;
use std::io::Seek;
use std::fs::File;
use byteorder::{BigEndian, ReadBytesExt};

extern crate num;
#[macro_use]
extern crate num_derive;
use num::FromPrimitive;

/*#[derive(FromPrimitive)]
enum ChannelEvent {
	NoteOff = 0x8,
	NoteOn = 0x9,
	NoteAftertouch = 0xa,
	Controller = 0xb,
	ProgramChange = 0xc,
	ChannelAftertouch = 0xd,
	PitchBend = 0xe,
}

#[derive(FromPrimitive)]
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
}*/

enum EventClass {
	Channel,
	SysEx,
	Meta,
}

#[derive(FromPrimitive)]
enum EventType {
	// Channel events
	NoteOff = 0x8,
	NoteOn = 0x9,
	NoteAftertouch = 0xa,
	Controller = 0xb,
	ProgramChange = 0xc,
	ChannelAftertouch = 0xd,
	PitchBend = 0xe,

	// Meta events
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

	// SysEx events
	SysEx = 0x4b,
}


struct Event {
	class: EventClass,
	delta: u32,
	status: u8,
	params: Vec<u8>,
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


fn read_channel_event(delta: u32, status: u8, event_type: u8, file: &mut File, byte_count: &mut u32) -> Event {
	let typ: EventType = FromPrimitive::from_u8(event_type).unwrap();
	let mut params = Vec::<u8>::new();
	let param_count = match typ {
		EventType::NoteOff => {
			2
		},
		EventType::NoteOn => {
			2
		},
		EventType::NoteAftertouch => {
			2
		},
		EventType::Controller => {
			2
		},

		EventType::ProgramChange => {
			1
		},
		EventType::ChannelAftertouch => {
			1
		},
		EventType::PitchBend => {
			2
		},
		_ => {
			panic!("Attempted to read non-channel event as a channel event");
		},
	};

	for _i in 0..param_count {
		params.push(file.read_u8().unwrap());
		*byte_count += 1;
	}

	Event{class: EventClass::Channel, delta, status, params}
}


fn read_meta_event(delta: u32, event_type: u8, file: &mut File, byte_count: &mut u32) -> Event {
	let length = file.read_u8().unwrap();
	*byte_count += 1;
	let mut params = Vec::<u8>::new();
	for _i in 0..length {
		params.push(file.read_u8().unwrap());
		*byte_count += 1;
	}

	//println!("Type 0x{:x}", event_type);
	//println!("Length: {}", length);
	//println!("Parameters: {:?}\n", params);

	Event{class: EventClass::Meta, delta, status: event_type, params}
}


fn read_event(file: &mut File, byte_count: &mut u32, prev_status: &mut u8) -> Event {
	let delta = read_vlq(file, byte_count);
	//println!("Delta {}", delta);

	let status = file.read_u8().unwrap();
	*byte_count += 1;
	match status {
		0xff => {
			//println!("Meta event");
			let typ = file.read_u8().unwrap();
			*byte_count += 1;
			read_meta_event(delta, typ, file, byte_count)
		},
		0xf0 => {
			//println!("SysEx event");
			Event{class: EventClass::SysEx, delta: 0, status: 0x0, params: vec![]}
		},
		0xf7 => {
			//println!("SysEx event (end)");
			Event{class: EventClass::SysEx, delta: 0, status: 0x0, params: vec![]}
		},
		_ => {
			//println!("Normal event");

			let typ: u8;
			// Running status:
			// If the high bit isn't set, we're reusing the same
			// status byte from the last event.
			if status < 0x80 {
				typ = high4(*prev_status);
				file.seek(SeekFrom::Current(-1)).unwrap();
				*byte_count -= 1;
			}
			else {
				typ = high4(status);
				*prev_status = status;
			}

			//println!("Type: 0x{:x}\nChannel: {}", typ, channel);

			read_channel_event(delta, typ, if status < 0x80 {*prev_status} else {status}, file, byte_count)
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

	let mut events = Vec::<Vec::<Event>>::new();

	for track in 0..tracks {
		//println!("Track {}", track);
		events.push(Vec::<Event>::new());

		let this_chunk_id = read_string(&mut file, 4);
		if this_chunk_id != "MTrk" {
			eprintln!("Error: Invalid track format");
			process::exit(1);
		}

		let this_chunk_size = file.read_u32::<BigEndian>().unwrap();
		//println!("{} bytes\n", this_chunk_size);

		let mut byte_count = 0;
		let mut status: u8 = 0x0;
		while byte_count < this_chunk_size {
			events[track as usize].push(read_event(&mut file, &mut byte_count, &mut status));
		}
	}
}
