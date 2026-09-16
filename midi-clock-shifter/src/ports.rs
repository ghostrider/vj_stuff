use std::time::Instant;

use crossbeam_channel::Sender;
use midir::{MidiInput, MidiInputConnection, MidiOutput};

use crate::engine::{InputEvent, RtMsg};

#[derive(Debug, thiserror::Error)]
pub enum HardwareInputError {
    #[error("failed to initialize MIDI input: {0}")]
    Init(#[from] midir::InitError),
    #[error("port '{0}' not found")]
    PortNotFound(String),
    #[error("failed to connect to port '{0}': {1}")]
    Connect(String, String),
}

/// Strips a Windows-appended duplicate-device-name prefix (`"2- "`, `"3- "`, …)
/// if present, otherwise returns `name` unchanged (ARCHITECTURE.md §7: "Windows
/// may append indices to duplicate device names").
fn strip_index_prefix(name: &str) -> &str {
    match name.split_once("- ") {
        Some((digits, rest))
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) =>
        {
            rest
        }
        _ => name,
    }
}

/// Finds which of `available` currently corresponds to the configured/selected
/// `name`: an exact match first, then one that only differs by a `N- ` index
/// prefix (ARCHITECTURE.md §7: "match by exact name first, then by name with a
/// leading `N- ` prefix stripped"). Settings stay keyed by `name` either way —
/// this only decides which live OS port to actually open.
pub fn resolve_port_name<'a>(name: &str, available: &'a [String]) -> Option<&'a str> {
    available
        .iter()
        .find(|candidate| candidate.as_str() == name)
        .or_else(|| {
            available
                .iter()
                .find(|candidate| strip_index_prefix(candidate) == name)
        })
        .map(String::as_str)
}

/// All currently available `midir` input port names, with no exclusions.
/// Unlike [`list_input_ports`], the Measure panel (PROMPTS.md Prompt 7)
/// deliberately *wants* to be able to select the app's own `Clock Shifter
/// Out` here (looped back, with no external cable/loopMIDI hop, to validate
/// real-world lead) — that's not the feedback-loop risk the main input
/// selection's exclusion guards against, since a measure input never feeds
/// back into the clock pipeline.
pub fn list_all_input_ports() -> Vec<String> {
    match MidiInput::new("midi_clock_shifter probe") {
        Ok(midi_in) => midi_in
            .ports()
            .iter()
            .filter_map(|port| midi_in.port_name(port).ok())
            .collect(),
        Err(err) => {
            log::warn!("failed to enumerate MIDI input ports: {err}");
            Vec::new()
        }
    }
}

/// Lists the names of all currently available hardware/OS MIDI input ports, except
/// `exclude` (our own `Clock Shifter Out` virtual port, which appears as a MIDI
/// *input* to the rest of the system but must never be selectable as our own input
/// source — that would be a feedback loop; ARCHITECTURE.md §7).
pub fn list_input_ports(exclude: &str) -> Vec<String> {
    match MidiInput::new("midi_clock_shifter probe") {
        Ok(midi_in) => midi_in
            .ports()
            .iter()
            .filter_map(|port| midi_in.port_name(port).ok())
            .filter(|name| name != exclude)
            .collect(),
        Err(err) => {
            log::warn!("failed to enumerate MIDI input ports: {err}");
            Vec::new()
        }
    }
}

/// Lists the names of all currently available hardware/OS MIDI output ports, except
/// `exclude` (our own `Clock Shifter In` virtual port, which appears as a MIDI
/// *output* to the rest of the system but must never be selectable as one of our
/// outputs; ARCHITECTURE.md §7).
pub fn list_output_ports(exclude: &str) -> Vec<String> {
    match MidiOutput::new("midi_clock_shifter probe") {
        Ok(midi_out) => midi_out
            .ports()
            .iter()
            .filter_map(|port| midi_out.port_name(port).ok())
            .filter(|name| name != exclude)
            .collect(),
        Err(err) => {
            log::warn!("failed to enumerate MIDI output ports: {err}");
            Vec::new()
        }
    }
}

/// Opens `name` as a hardware MIDI clock input, forwarding realtime bytes into
/// `sender` as [`InputEvent`]s.
///
/// Filters per byte, not per message: real-time bytes (`0xF8`/`0xFA`/`0xFB`/
/// `0xFC`) can be interleaved inside other MIDI messages, so every byte in a
/// callback's data is checked independently rather than assuming byte 0 is a
/// status byte for the whole message. Anything else, including `0xFE` active
/// sensing, is ignored (midi-clock-protocol skill).
pub fn open_hardware_input(
    name: &str,
    sender: Sender<InputEvent>,
) -> Result<MidiInputConnection<()>, HardwareInputError> {
    let midi_in = MidiInput::new("midi_clock_shifter input")?;
    let ports = midi_in.ports();
    let available: Vec<String> = ports
        .iter()
        .filter_map(|p| midi_in.port_name(p).ok())
        .collect();
    let resolved = resolve_port_name(name, &available)
        .ok_or_else(|| HardwareInputError::PortNotFound(name.to_string()))?;
    let port = ports
        .iter()
        .find(|p| midi_in.port_name(p).as_deref() == Ok(resolved))
        .ok_or_else(|| HardwareInputError::PortNotFound(name.to_string()))?;

    midi_in
        .connect(
            port,
            "midi_clock_shifter-in",
            move |_stamp_us, data, _| {
                // Timestamp first, before anything else (ARCHITECTURE.md §3 Threads).
                let now = Instant::now();
                for &byte in data {
                    if let Some(msg) = RtMsg::from_status_byte(byte) {
                        // No allocation, no blocking: silently drop on overflow.
                        let _ = sender.try_send(InputEvent { at: now, msg });
                    }
                }
            },
            (),
        )
        .map_err(|err| HardwareInputError::Connect(name.to_string(), err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_an_exact_match_first() {
        let available = vec!["Beatstep Pro".to_string(), "2- Beatstep Pro".to_string()];
        assert_eq!(
            resolve_port_name("Beatstep Pro", &available),
            Some("Beatstep Pro")
        );
    }

    #[test]
    fn resolves_a_replugged_device_via_the_stripped_prefix() {
        let available = vec!["2- Beatstep Pro".to_string()];
        assert_eq!(
            resolve_port_name("Beatstep Pro", &available),
            Some("2- Beatstep Pro")
        );
    }

    #[test]
    fn does_not_resolve_an_unrelated_name() {
        let available = vec!["2- Beatstep Pro".to_string()];
        assert_eq!(resolve_port_name("KeyStep", &available), None);
    }

    #[test]
    fn does_not_strip_a_name_that_merely_contains_a_dash() {
        // A real device name containing "- " but not a pure numeric prefix
        // must not be mistaken for an indexed duplicate.
        let available = vec!["Arturia - KeyStep".to_string()];
        assert_eq!(resolve_port_name("KeyStep", &available), None);
        assert_eq!(
            resolve_port_name("Arturia - KeyStep", &available),
            Some("Arturia - KeyStep")
        );
    }
}
