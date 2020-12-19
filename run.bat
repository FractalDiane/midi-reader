@echo off
if [%2]==[] (
	cargo run %1
) else (
	if [%2] == [debug] (
		cargo build --features midi_debug
		target\debug\midi_reader.exe %1
	) else (
		cargo run %1
	)
)
