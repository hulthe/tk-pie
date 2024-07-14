A DIY split keyboard.

## Directory

### `/lib`
A rust library, and the bulk of the keyboard firmware implementation.
Written using the Embassy framework.

### `/left` and `/right`
Rust binaries, one for each half of the split keyboard.
Most of the actual logic is implemented in `/lib`.

### `/schematic`
Kicad schematics of the keyboard PCBs.

### `/case`
3d-printable files of the keyboard case and related plastics.

### `/editor`
A graphical program for configuring the keyboard layout. Work in progress.


## Developing

Required tools:
- Rust, obviously.
- `probe-rs` for flashing the firmware. Follow instructions at https://probe.rs/
- [`flip-link`](https://github.com/knurling-rs/flip-link/)
- A debug probe of some kind, I recommend a [Pitaya-Link](https://github.com/makerdiary/pitaya-link)

To flash:
- plug in the probe,
- `cd` into either `left` or `right`
- `cargo run`
