//! C FFI bindings for native application developers.
//!
//! Link against `archergate_license.lib` (Windows), `libarchergate_license.a` (macOS/Linux),
//! or the shared library (`archergate_license.dll` / `.dylib` / `.so`).
//!
//! All functions are safe to call from any thread.
//! The client pointer must be freed with [`ag_license_free`].

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::client::LicenseClient;

/// Opaque handle to a license client. Created by [`ag_license_new`], freed by [`ag_license_free`].
pub type AgLicenseClient = LicenseClient;

/// Create a new license client.
///
/// Returns a heap-allocated client pointer. Must be freed with [`ag_license_free`].
/// Returns `null` if either argument is null or not valid UTF-8.
///
/// # Safety
/// `api_key` and `plugin_id` must be valid, null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_new(
    api_key: *const c_char,
    plugin_id: *const c_char,
) -> *mut AgLicenseClient {
    let api_key = match unsafe { ptr_to_str(api_key) } {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let plugin_id = match unsafe { ptr_to_str(plugin_id) } {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(LicenseClient::new(api_key, plugin_id)))
}

/// Create a new license client with a custom API URL.
///
/// Same as [`ag_license_new`] but overrides the server URL.
///
/// # Safety
/// All pointer arguments must be valid, null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_new_with_url(
    api_key: *const c_char,
    plugin_id: *const c_char,
    api_url: *const c_char,
) -> *mut AgLicenseClient {
    let api_key = match unsafe { ptr_to_str(api_key) } {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let plugin_id = match unsafe { ptr_to_str(plugin_id) } {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let api_url = match unsafe { ptr_to_str(api_url) } {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(
        LicenseClient::new(api_key, plugin_id).with_api_url(api_url),
    ))
}

/// Validate a license key.
///
/// Returns `0` on success, or a negative error code:
/// - `-1` = Invalid key
/// - `-2` = Expired
/// - `-3` = Machine mismatch
/// - `-4` = Network error (and no offline cache)
/// - `-5` = Trial expired
/// - `-6` = Activation limit reached
///
/// Use [`ag_license_error_string`] to get a human-readable message.
///
/// # Safety
/// `client` must be a valid pointer from [`ag_license_new`].
/// `license_key` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_validate(
    client: *const AgLicenseClient,
    license_key: *const c_char,
) -> i32 {
    let client = match unsafe { client.as_ref() } {
        Some(c) => c,
        None => return -1,
    };
    let key = match unsafe { ptr_to_str(license_key) } {
        Some(s) => s,
        None => return -1,
    };
    match client.validate(key) {
        Ok(()) => 0,
        Err(e) => e.to_code(),
    }
}

/// Activate a license on this machine.
///
/// Returns `0` on success, or a negative error code (same as [`ag_license_validate`]).
///
/// # Safety
/// `client` must be a valid pointer from [`ag_license_new`].
/// `license_key` and `email` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_activate(
    client: *const AgLicenseClient,
    license_key: *const c_char,
    email: *const c_char,
) -> i32 {
    let client = match unsafe { client.as_ref() } {
        Some(c) => c,
        None => return -1,
    };
    let key = match unsafe { ptr_to_str(license_key) } {
        Some(s) => s,
        None => return -1,
    };
    let email = match unsafe { ptr_to_str(email) } {
        Some(s) => s,
        None => return -1,
    };
    match client.activate(key, email) {
        Ok(_) => 0,
        Err(e) => e.to_code(),
    }
}

/// Start a 14-day trial.
///
/// Returns `0` on success, or a negative error code.
/// On success, writes the number of remaining trial days to `*out_days`.
///
/// # Safety
/// `client` must be a valid pointer from [`ag_license_new`].
/// `out_days` must be a valid pointer to a `u32`, or null (days won't be written).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_start_trial(
    client: *const AgLicenseClient,
    out_days: *mut u32,
) -> i32 {
    let client = match unsafe { client.as_ref() } {
        Some(c) => c,
        None => return -5,
    };
    match client.start_trial() {
        Ok(trial) => {
            if !out_days.is_null() {
                unsafe { *out_days = trial.days_remaining };
            }
            0
        }
        Err(e) => e.to_code(),
    }
}

/// Write the machine fingerprint into `buf` (64 hex chars + null terminator = 65 bytes min).
///
/// Returns `0` on success, `-1` if `buf` is null or `buf_len < 65`.
///
/// # Safety
/// `buf` must point to at least `buf_len` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_fingerprint(buf: *mut c_char, buf_len: usize) -> i32 {
    if buf.is_null() || buf_len < 65 {
        return -1;
    }
    let fp = LicenseClient::machine_fingerprint();
    let bytes = fp.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, 64);
        *buf.add(64) = 0; // null terminator
    }
    0
}

/// Get a human-readable error message for an error code.
///
/// Returns a static string. Never returns null.
#[unsafe(no_mangle)]
pub extern "C" fn ag_license_error_string(code: i32) -> *const c_char {
    match code {
        0 => c"success".as_ptr(),
        -1 => c"license key is invalid".as_ptr(),
        -2 => c"license has expired".as_ptr(),
        -3 => c"license is bound to a different machine".as_ptr(),
        -4 => c"network error (no internet and no cached license)".as_ptr(),
        -5 => c"trial period has expired".as_ptr(),
        -6 => c"activation limit reached for this license".as_ptr(),
        _ => c"unknown error".as_ptr(),
    }
}

/// Free a license client created by [`ag_license_new`].
///
/// Safe to call with null (no-op). Must not be called twice on the same pointer.
///
/// # Safety
/// `client` must be a pointer returned by [`ag_license_new`] that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ag_license_free(client: *mut AgLicenseClient) {
    if !client.is_null() {
        drop(unsafe { Box::from_raw(client) });
    }
}

/// # Safety
/// `ptr` must be a valid, null-terminated C string or null.
unsafe fn ptr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}
