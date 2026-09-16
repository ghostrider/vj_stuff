//! `ClockOutput` trait + `VirtualOut` / `MidirOut` implementations (ARCHITECTURE.md §6).
//!
//! Kept a small, concrete-crate-facing layer: the engine thread only ever talks
//! to outputs through the [`ClockOutput`] trait, never `vmidi`/`midir` directly.

use std::sync::Arc;

use midir::{MidiOutput, MidiOutputConnection};

use crate::vmidi::VmidiError;
use crate::vmidi::ffi::VmidiApi;

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error(transparent)]
    Vmidi(#[from] VmidiError),
    #[error("failed to initialize MIDI output: {0}")]
    Init(String),
    #[error("port '{0}' not found")]
    PortNotFound(String),
    #[error("failed to connect to port '{0}': {1}")]
    Connect(String, String),
    #[error("failed to send on '{0}': {1}")]
    Send(String, String),
}

/// A destination the engine can send realtime clock bytes to (ARCHITECTURE.md §6).
pub trait ClockOutput: Send {
    fn name(&self) -> &str;
    fn send(&mut self, msg: &[u8]) -> Result<(), OutputError>;
    fn is_connected(&self) -> bool;
}

/// The `Clock Shifter Out` virtual port.
pub struct VirtualOut {
    port: crate::vmidi::VirtualPort,
}

impl VirtualOut {
    pub fn create(api: Arc<VmidiApi>, name: &str) -> Result<Self, OutputError> {
        let port = crate::vmidi::VirtualPort::create_output(api, name)?;
        Ok(Self { port })
    }
}

impl ClockOutput for VirtualOut {
    fn name(&self) -> &str {
        self.port.name()
    }

    fn send(&mut self, msg: &[u8]) -> Result<(), OutputError> {
        self.port.send(msg).map_err(OutputError::from)
    }

    fn is_connected(&self) -> bool {
        self.port.is_open()
    }
}

/// A hardware (or other OS-visible) MIDI output port, via `midir`.
pub struct MidirOut {
    name: String,
    connection: MidiOutputConnection,
    /// Reflects whether the *last* send succeeded; `midir`/WinMM don't offer an
    /// active "still connected" check, so this is the best available proxy until
    /// hot-plug detection exists (Prompt 6).
    healthy: bool,
}

impl MidirOut {
    pub fn open(name: &str) -> Result<Self, OutputError> {
        let midi_out = MidiOutput::new("midi_clock_shifter output")
            .map_err(|err| OutputError::Init(err.to_string()))?;
        let ports = midi_out.ports();
        let available: Vec<String> = ports
            .iter()
            .filter_map(|p| midi_out.port_name(p).ok())
            .collect();
        // ARCHITECTURE.md §7: match by exact name first, then by name with a
        // leading `N- ` duplicate-index prefix stripped (Windows may rename a
        // replugged device that way).
        let resolved = crate::ports::resolve_port_name(name, &available)
            .ok_or_else(|| OutputError::PortNotFound(name.to_string()))?;
        let port = ports
            .iter()
            .find(|p| midi_out.port_name(p).as_deref() == Ok(resolved))
            .ok_or_else(|| OutputError::PortNotFound(name.to_string()))?;
        let connection = midi_out
            .connect(port, "midi_clock_shifter-out")
            .map_err(|err| OutputError::Connect(name.to_string(), err.to_string()))?;

        Ok(Self {
            name: name.to_string(),
            connection,
            healthy: true,
        })
    }
}

impl ClockOutput for MidirOut {
    fn name(&self) -> &str {
        &self.name
    }

    fn send(&mut self, msg: &[u8]) -> Result<(), OutputError> {
        match self.connection.send(msg) {
            Ok(()) => {
                self.healthy = true;
                Ok(())
            }
            Err(err) => {
                self.healthy = false;
                Err(OutputError::Send(self.name.clone(), err.to_string()))
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.healthy
    }
}
