//! Raw bindings to `teVirtualMIDI64.dll`, loaded at runtime via `libloading`.
//!
//! Declarations are taken verbatim from `teVirtualMIDI.h` (virtualMIDI SDK v1.3.0.43,
//! Tobias Erichsen) — do not change a signature or flag value without checking that
//! header again.

use libloading::Library;
use windows::core::BOOL;

pub const DLL_NAME: &str = "teVirtualMIDI64.dll";

/// Bits for the `flags` parameter of `virtualMIDICreatePortEx2` (teVirtualMIDI.h).
/// TE_VM_FLAGS_PARSE_RX: driver always provides valid preparsed MIDI commands.
pub const TE_VM_FLAGS_PARSE_RX: u32 = 1;
/// TE_VM_FLAGS_PARSE_TX: driver parses all data sent via `virtualMIDISendData`.
pub const TE_VM_FLAGS_PARSE_TX: u32 = 2;
/// TE_VM_FLAGS_INSTANTIATE_RX_ONLY: only the "midi-out" part is created (other apps
/// see it as a MIDI output they can send to; we only receive).
pub const TE_VM_FLAGS_INSTANTIATE_RX_ONLY: u32 = 4;
/// TE_VM_FLAGS_INSTANTIATE_TX_ONLY: only the "midi-in" part is created (other apps
/// see it as a MIDI input they can read from; we only send).
pub const TE_VM_FLAGS_INSTANTIATE_TX_ONLY: u32 = 8;

/// Opaque `VM_MIDI_PORT` (`typedef struct _VM_MIDI_PORT VM_MIDI_PORT, *LPVM_MIDI_PORT;`).
#[repr(C)]
pub struct VmMidiPort {
    _private: [u8; 0],
}

pub type LpVmMidiPort = *mut VmMidiPort;

/// `typedef void (CALLBACK *LPVM_MIDI_DATA_CB)(LPVM_MIDI_PORT, LPBYTE, DWORD, DWORD_PTR);`
///
/// Called by the driver on an arbitrary thread. `midi_data_bytes` is null and `length`
/// is zero exactly once, when the driver is deactivated (port closed from the driver
/// side) — see teVirtualMIDI.h's callback documentation.
pub type VmMidiDataCb = unsafe extern "system" fn(
    midi_port: LpVmMidiPort,
    midi_data_bytes: *mut u8,
    length: u32,
    dw_callback_instance: usize,
);

type FnCreatePortEx2 = unsafe extern "system" fn(
    port_name: *const u16,
    callback: Option<VmMidiDataCb>,
    dw_callback_instance: usize,
    max_sysex_length: u32,
    flags: u32,
) -> LpVmMidiPort;

type FnClosePort = unsafe extern "system" fn(midi_port: LpVmMidiPort);

type FnShutdown = unsafe extern "system" fn(midi_port: LpVmMidiPort) -> BOOL;

type FnSendData = unsafe extern "system" fn(
    midi_port: LpVmMidiPort,
    midi_data_bytes: *const u8,
    length: u32,
) -> BOOL;

type FnGetVersion = unsafe extern "system" fn(
    major: *mut u16,
    minor: *mut u16,
    release: *mut u16,
    build: *mut u16,
) -> *const u16;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("could not load {0}: {1}")]
    Library(&'static str, #[source] libloading::Error),
    #[error("symbol {0} not found in {1}: {2}")]
    Symbol(&'static str, &'static str, #[source] libloading::Error),
}

/// Resolved function pointers into `teVirtualMIDI64.dll`. Keeps the [`Library`] alive
/// for as long as any of them may still be called.
pub struct VmidiApi {
    _lib: Library,
    create_port_ex2_fn: FnCreatePortEx2,
    close_port_fn: FnClosePort,
    shutdown_fn: FnShutdown,
    send_data_fn: FnSendData,
    get_version_fn: FnGetVersion,
    get_driver_version_fn: FnGetVersion,
}

impl VmidiApi {
    /// Loads `teVirtualMIDI64.dll` from the standard DLL search path (it is installed
    /// system-wide by the virtualMIDI driver / loopMIDI installer) and resolves every
    /// symbol used by this app. Fails clearly if the DLL or a symbol is missing;
    /// callers must let the app start anyway and show an error banner instead of
    /// panicking (ARCHITECTURE.md §2, "Prerequisite: virtualMIDI driver").
    pub fn load() -> Result<Self, LoadError> {
        // Safety: `DLL_NAME` is loaded by name from the standard search path; the
        // driver DLL does not run initialization code that is unsafe to call here.
        let lib =
            unsafe { Library::new(DLL_NAME) }.map_err(|err| LoadError::Library(DLL_NAME, err))?;

        let create_port_ex2_fn =
            unsafe { load_symbol::<FnCreatePortEx2>(&lib, "virtualMIDICreatePortEx2") }?;
        let close_port_fn = unsafe { load_symbol::<FnClosePort>(&lib, "virtualMIDIClosePort") }?;
        let shutdown_fn = unsafe { load_symbol::<FnShutdown>(&lib, "virtualMIDIShutdown") }?;
        let send_data_fn = unsafe { load_symbol::<FnSendData>(&lib, "virtualMIDISendData") }?;
        let get_version_fn = unsafe { load_symbol::<FnGetVersion>(&lib, "virtualMIDIGetVersion") }?;
        let get_driver_version_fn =
            unsafe { load_symbol::<FnGetVersion>(&lib, "virtualMIDIGetDriverVersion") }?;

        Ok(Self {
            _lib: lib,
            create_port_ex2_fn,
            close_port_fn,
            shutdown_fn,
            send_data_fn,
            get_version_fn,
            get_driver_version_fn,
        })
    }

    /// # Safety
    /// `port_name` must point to a valid, null-terminated UTF-16 string for the
    /// duration of the call. If `callback` is `Some`, it must remain valid (and safe
    /// to call from an arbitrary thread) until the returned port is closed, and
    /// `dw_callback_instance` must be a value that callback can safely interpret.
    pub unsafe fn create_port_ex2(
        &self,
        port_name: *const u16,
        callback: Option<VmMidiDataCb>,
        dw_callback_instance: usize,
        max_sysex_length: u32,
        flags: u32,
    ) -> LpVmMidiPort {
        unsafe {
            (self.create_port_ex2_fn)(
                port_name,
                callback,
                dw_callback_instance,
                max_sysex_length,
                flags,
            )
        }
    }

    /// # Safety
    /// `port` must be a currently open handle returned by [`Self::create_port_ex2`],
    /// must not be used again afterwards, and this must not be called from within
    /// that port's own data callback (teVirtualMIDI.h: "may result in a deadlock").
    pub unsafe fn close_port(&self, port: LpVmMidiPort) {
        unsafe { (self.close_port_fn)(port) }
    }

    /// Aborts a port, cancelling any pending activity, before it is closed.
    ///
    /// # Safety
    /// `port` must be a currently open handle. After this call, only
    /// [`Self::close_port`] may still be called on it (teVirtualMIDI.h).
    pub unsafe fn shutdown(&self, port: LpVmMidiPort) -> bool {
        unsafe { (self.shutdown_fn)(port) }.as_bool()
    }

    /// # Safety
    /// `port` must be a currently open handle. `data` must point to at least `length`
    /// valid, readable bytes forming a single, complete MIDI command.
    pub unsafe fn send_data(&self, port: LpVmMidiPort, data: *const u8, length: u32) -> bool {
        unsafe { (self.send_data_fn)(port, data, length) }.as_bool()
    }

    /// DLL (interface) version, e.g. `"1.3.0.43"`, or `None` if unavailable.
    pub fn version_string(&self) -> Option<String> {
        // Safety: all four output pointers are null, which teVirtualMIDI.h documents
        // as valid (the caller only wants the returned version string).
        let ptr = unsafe {
            (self.get_version_fn)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        unsafe { wide_c_str_to_string(ptr) }
    }

    /// Kernel driver version, e.g. `"1.3.0.43"`, or `None` if unavailable.
    pub fn driver_version_string(&self) -> Option<String> {
        let ptr = unsafe {
            (self.get_driver_version_fn)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        unsafe { wide_c_str_to_string(ptr) }
    }
}

/// # Safety
/// `lib` must have `symbol` exported with a signature compatible with `T`.
unsafe fn load_symbol<T: Copy>(lib: &Library, symbol: &'static str) -> Result<T, LoadError> {
    let sym: libloading::Symbol<'_, T> =
        unsafe { lib.get(symbol) }.map_err(|err| LoadError::Symbol(symbol, DLL_NAME, err))?;
    Ok(*sym)
}

/// # Safety
/// `ptr` must be null or point to a valid, null-terminated UTF-16 string that stays
/// valid for the duration of this call.
unsafe fn wide_c_str_to_string(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(
            ptr, len,
        )))
    }
}
