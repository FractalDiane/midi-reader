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

#[derive(FromPrimitive)]
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
	while buffer >> 7 == 1 {
		buffer = file.read_u8().unwrap() as u32;
		result |= (buffer & 0x7f) << (7 * i);
		*byte_count += 1;
		i += 1;
	}

	result
}


fn read_channel_event(event_type: u8, file: &mut File, byte_count: &mut u32) {
	let typ: ChannelEvent = FromPrimitive::from_u8(event_type).unwrap();
	match typ {
		ChannelEvent::NoteOff => {
			let note = file.read_u8().unwrap();
			let velocity = file.read_u8().unwrap();
			println!("Note Off\nNote: {}\nVelocity: {}\n", note, velocity);
			*byte_count += 2;
		},

		ChannelEvent::NoteOn => {
			let note = file.read_u8().unwrap();
			let velocity = file.read_u8().unwrap();
			println!("Note On\nNote: {}\nVelocity: {}\n", note, velocity);
			*byte_count += 2;
		},

		ChannelEvent::NoteAftertouch => {
			let note = file.read_u8().unwrap();
			let pressure = file.read_u8().unwrap();
			println!("Note Aftertouch\nNote: {}\nPressure: {}\n", note, pressure);
			*byte_count += 2;
		},

		ChannelEvent::Controller => {
			let controller = file.read_u8().unwrap();
			let value = file.read_u8().unwrap();
			println!("Controller change\nController: {}\nValue: {}\n", controller, value);
			*byte_count += 2;
		},

		ChannelEvent::ProgramChange => {
			let number = file.read_u8().unwrap();
			println!("Program change\nProgram: {}\n", number);
			*byte_count += 1;
		},

		ChannelEvent::ChannelAftertouch => {
			let pressure = file.read_u8().unwrap();
			println!("Channel aftertouch\nPressure: {}\n", pressure);
			*byte_count += 1;
		},

		ChannelEvent::PitchBend => {
			let low = file.read_u8().unwrap() as u16;
			let high = file.read_u8().unwrap() as u16;
			println!("Pitch bend\nValue: {}\n", (high << 8) | low);
			*byte_count += 2;
		},
	}
}


fn read_meta_event(event_type: u8, file: &mut File, byte_count: &mut u32) {
	let typ: MetaEvent = FromPrimitive::from_u8(event_type).unwrap();
	let length = file.read_u8().unwrap() as u32;
	*byte_count += 1;
	let mut params = Vec::<u8>::new();
	for _i in 0..length {
		params.push(file.read_u8().unwrap());
		*byte_count += 1;
	}

	println!("Type 0x{:x}", event_type);
	println!("Length: {}", length);
	println!("Parameters: {:?}\n", params);
}


fn read_event(file: &mut File, byte_count: &mut u32, prev_status: &mut u8) {
	let delta = read_vlq(file, byte_count);
	println!("Delta {}", delta);

	let status = file.read_u8().unwrap();
	*byte_count += 1;
	match status {
		0xff => {
			println!("Meta event");
			let typ = file.read_u8().unwrap();
			*byte_count += 1;
			read_meta_event(typ, file, byte_count);
		},
		0xf0 => {
			println!("SysEx event");
		},
		0xf7 => {
			println!("SysEx event (end)");
		},
		_ => {
			println!("Normal event");

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

			println!("Type: 0x{:x}\nChannel: {}", typ, channel);

			read_channel_event(typ, file, byte_count);
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
	
	for track in 0..tracks {
		println!("Track {}", track);

		let this_chunk_id = read_string(&mut file, 4);
		if this_chunk_id != "MTrk" {
			eprintln!("Error: Invalid track format");
			process::exit(1);
		}

		let this_chunk_size = file.read_u32::<BigEndian>().unwrap();
		println!("{} bytes\n", this_chunk_size);

		let mut byte_count = 0;
		let mut status: u8 = 0x0;
		while byte_count < this_chunk_size {
			read_event(&mut file, &mut byte_count, &mut status);
		}
	}
}
