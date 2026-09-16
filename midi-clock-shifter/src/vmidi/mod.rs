//! Safe wrapper around the virtualMIDI SDK (ARCHITECTURE.md §4): [`VirtualPort`]
//! create/send/close/Drop, backed by the raw bindings in [`ffi`].

pub mod ffi;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use crossbeam_channel::Sender;
use windows::Win32::Foundation::GetLastError;

use crate::engine::{InputEvent, RtMsg};
use ffi::{LpVmMidiPort, VmidiApi};

#[derive(Debug, thiserror::Error)]
pub enum VmidiError {
    #[error(transparent)]
    Load(#[from] ffi::LoadError),
    #[error("port name '{0}' contains an embedded NUL character")]
    InvalidName(String),
    #[error("virtualMIDICreatePortEx2 failed for port '{name}' (GetLastError={code})")]
    CreatePortFailed { name: String, code: u32 },
    #[error("virtualMIDISendData failed on port '{name}' (GetLastError={code})")]
    SendFailed { name: String, code: u32 },
}

/// Raw message counters, updated lock-free from the driver callback. Temporary
/// debug aid (Prompt 2); the estimator (Prompt 3+) is the real source of timing
/// information now.
#[derive(Debug, Default)]
pub struct PortCounters {
    pub clock: AtomicU64,
    pub start: AtomicU64,
    pub continue_msgs: AtomicU64,
    pub stop: AtomicU64,
    pub other: AtomicU64,
    pub overflow: AtomicU64,
    pub closed: AtomicBool,
}

struct CallbackContext {
    sender: Option<Sender<InputEvent>>,
    counters: Arc<PortCounters>,
}

/// A created virtual MIDI port (either `Clock Shifter In` or `Clock Shifter Out`).
///
/// Owns the port handle and (if created with a channel) the boxed [`CallbackContext`]
/// the driver calls back into; both are released in [`Drop`].
pub struct VirtualPort {
    handle: LpVmMidiPort,
    api: Arc<VmidiApi>,
    name: String,
    context_ptr: *mut CallbackContext,
    pub counters: Arc<PortCounters>,
}

// Safety: the driver documents that the data callback "is called in an arbitrary
// thread-context", and `virtualMIDIClosePort`/`virtualMIDISendData` are safe to call
// from any thread as long as they're not called concurrently with each other on the
// same port from multiple threads without synchronization (we only ever call them
// from the owning thread after moving `VirtualPort` there).
unsafe impl Send for VirtualPort {}

impl VirtualPort {
    /// Creates `Clock Shifter In`-style port: RX-only (appears to other apps as a
    /// MIDI *output* they can send to), RX-parsed, delivering [`InputEvent`]s into
    /// `sender` from the driver callback.
    pub fn create_input(
        api: Arc<VmidiApi>,
        name: &str,
        max_sysex_length: u32,
        sender: Sender<InputEvent>,
    ) -> Result<Self, VmidiError> {
        let flags = ffi::TE_VM_FLAGS_PARSE_RX | ffi::TE_VM_FLAGS_INSTANTIATE_RX_ONLY;
        Self::create(api, name, flags, max_sysex_length, Some(sender))
    }

    /// Creates a `Clock Shifter Out`-style port: TX-only (appears to other apps as a
    /// MIDI *input* they can read from), TX-parsed. No inbound data is ever possible
    /// on a TX-only port, so no [`Sender`] is needed; a callback is still registered
    /// purely to catch the "port closed" signal.
    pub fn create_output(api: Arc<VmidiApi>, name: &str) -> Result<Self, VmidiError> {
        let flags = ffi::TE_VM_FLAGS_PARSE_TX | ffi::TE_VM_FLAGS_INSTANTIATE_TX_ONLY;
        Self::create(api, name, flags, 0, None)
    }

    fn create(
        api: Arc<VmidiApi>,
        name: &str,
        flags: u32,
        max_sysex_length: u32,
        sender: Option<Sender<InputEvent>>,
    ) -> Result<Self, VmidiError> {
        if name.contains('\0') {
            return Err(VmidiError::InvalidName(name.to_string()));
        }

        let counters = Arc::new(PortCounters::default());
        let context = Box::new(CallbackContext {
            sender,
            counters: Arc::clone(&counters),
        });
        let context_ptr = Box::into_raw(context);

        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        // Safety: `wide_name` is a valid null-terminated UTF-16 buffer, live for the
        // duration of this call. `port_data_callback` is a plain `'static` function,
        // safe to call from any thread. `context_ptr` is a live, uniquely-owned
        // `Box` leaked for exactly this purpose; it is only freed after
        // `virtualMIDIClosePort` returns (see `Drop`), by which point the driver is
        // guaranteed not to call back into it again.
        let handle = unsafe {
            api.create_port_ex2(
                wide_name.as_ptr(),
                Some(port_data_callback),
                context_ptr as usize,
                max_sysex_length,
                flags,
            )
        };

        if handle.is_null() {
            // Safety: creation failed, so the driver holds no reference to
            // `context_ptr` and will never call back with it; reclaim and drop it
            // here instead of leaking.
            let code = unsafe { GetLastError() }.0;
            drop(unsafe { Box::from_raw(context_ptr) });
            return Err(VmidiError::CreatePortFailed {
                name: name.to_string(),
                code,
            });
        }

        Ok(Self {
            handle,
            api,
            name: name.to_string(),
            context_ptr,
            counters,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// `true` unless the driver has signalled that this port was deactivated (see
    /// [`PortCounters::closed`]).
    pub fn is_open(&self) -> bool {
        !self.counters.closed.load(Ordering::Relaxed)
    }

    /// Sends a single, complete MIDI command (e.g. one realtime status byte).
    pub fn send(&self, data: &[u8]) -> Result<(), VmidiError> {
        // Safety: `self.handle` is a valid, open port for the lifetime of `self`;
        // `data` is a valid slice of `data.len()` readable bytes.
        let ok = unsafe {
            self.api
                .send_data(self.handle, data.as_ptr(), data.len() as u32)
        };
        if ok {
            Ok(())
        } else {
            let code = unsafe { GetLastError() }.0;
            Err(VmidiError::SendFailed {
                name: self.name.clone(),
                code,
            })
        }
    }
}

impl Drop for VirtualPort {
    fn drop(&mut self) {
        // Abort any pending driver-side activity first (teVirtualMIDI.h); this is
        // meant for unblocking a pending virtualMIDIGetData call, which we never
        // use, but it is a cheap, documented step before closing and can only help.
        // Safety: `self.handle` was returned by `create_port_ex2` and has not been
        // closed or shut down before.
        unsafe { self.api.shutdown(self.handle) };

        // Safety: `self.handle` was returned by `create_port_ex2` and has not been
        // closed before; after this call the driver guarantees no further callbacks.
        unsafe { self.api.close_port(self.handle) };

        if !self.context_ptr.is_null() {
            // Safety: `virtualMIDIClosePort` above guarantees no more callbacks, so
            // no other reference to this `CallbackContext` can be in use anymore.
            drop(unsafe { Box::from_raw(self.context_ptr) });
        }
    }
}

unsafe extern "system" fn port_data_callback(
    _port: LpVmMidiPort,
    midi_data_bytes: *mut u8,
    length: u32,
    dw_callback_instance: usize,
) {
    // Timestamp first, before anything else (ARCHITECTURE.md §3 Threads).
    let now = Instant::now();

    if dw_callback_instance == 0 {
        return;
    }
    // Safety: `dw_callback_instance` is always a `CallbackContext` pointer we
    // created in `VirtualPort::create` and freed only after the driver guaranteed
    // no more callbacks (see `Drop`), so it is valid for the duration of this call.
    let ctx = unsafe { &*(dw_callback_instance as *const CallbackContext) };

    if midi_data_bytes.is_null() || length == 0 {
        // Driver deactivated / port closed from the driver side (teVirtualMIDI.h).
        ctx.counters.closed.store(true, Ordering::Relaxed);
        return;
    }

    // Safety: `midi_data_bytes` points to at least `length >= 1` valid bytes; we
    // only ever read the first one (the status byte of the preparsed command).
    let status_byte = unsafe { *midi_data_bytes };
    let Some(msg) = RtMsg::from_status_byte(status_byte) else {
        ctx.counters.other.fetch_add(1, Ordering::Relaxed);
        return;
    };

    match msg {
        RtMsg::Clock => {
            ctx.counters.clock.fetch_add(1, Ordering::Relaxed);
        }
        RtMsg::Start => {
            ctx.counters.start.fetch_add(1, Ordering::Relaxed);
        }
        RtMsg::Continue => {
            ctx.counters.continue_msgs.fetch_add(1, Ordering::Relaxed);
        }
        RtMsg::Stop => {
            ctx.counters.stop.fetch_add(1, Ordering::Relaxed);
        }
    }

    if let Some(sender) = &ctx.sender {
        // No allocation, no blocking: a full/disconnected channel just counts as
        // overflow (ARCHITECTURE.md §3 Threads).
        if sender.try_send(InputEvent { at: now, msg }).is_err() {
            ctx.counters.overflow.fetch_add(1, Ordering::Relaxed);
        }
    }
}
