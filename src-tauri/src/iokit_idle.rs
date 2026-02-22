//! macOS IOKit FFI for reading system idle time (HIDIdleTime).
//!
//! Uses IOKit's IOHIDSystem to read nanoseconds since last keyboard/mouse input.
//! No Accessibility or other permissions required — HIDIdleTime is a public system property.

use std::ffi::CString;
use std::os::raw::c_char;

// IOKit FFI declarations
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceGetMatchingService(master_port: u32, matching: *const core::ffi::c_void) -> u32;
    fn IOServiceMatching(name: *const c_char) -> *mut core::ffi::c_void;
    fn IORegistryEntryCreateCFProperty(
        entry: u32,
        key: *const core::ffi::c_void,
        allocator: *const core::ffi::c_void,
        options: u32,
    ) -> *const core::ffi::c_void;
    fn IOObjectRelease(object: u32) -> i32;
}

// CoreFoundation FFI declarations
use core_foundation_sys::number::{kCFNumberSInt64Type, CFNumberGetValue};
use core_foundation_sys::string::CFStringCreateWithCString;
use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};

const K_IO_MASTER_PORT_DEFAULT: u32 = 0;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

/// Get the number of seconds since the last user input (keyboard/mouse).
///
/// Returns `Ok(seconds)` on success, `Err(description)` on failure.
/// This function is safe to call from any thread.
pub fn get_idle_seconds() -> Result<f64, String> {
    unsafe {
        // Find the IOHIDSystem service
        let service_name = CString::new("IOHIDSystem")
            .map_err(|e| format!("CString error: {}", e))?;
        let matching = IOServiceMatching(service_name.as_ptr());
        if matching.is_null() {
            return Err("IOServiceMatching returned null".to_string());
        }

        let service = IOServiceGetMatchingService(K_IO_MASTER_PORT_DEFAULT, matching);
        // Note: IOServiceMatching result is consumed by IOServiceGetMatchingService
        if service == 0 {
            return Err("IOHIDSystem service not found".to_string());
        }

        // Create CFString for "HIDIdleTime" property key
        let key_name = CString::new("HIDIdleTime")
            .map_err(|e| format!("CString error: {}", e))?;
        let cf_key = CFStringCreateWithCString(
            kCFAllocatorDefault,
            key_name.as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );
        if cf_key.is_null() {
            IOObjectRelease(service);
            return Err("Failed to create CFString for HIDIdleTime".to_string());
        }

        // Read the HIDIdleTime property (returns CFNumber in nanoseconds)
        let cf_value = IORegistryEntryCreateCFProperty(
            service,
            cf_key as *const core::ffi::c_void,
            kCFAllocatorDefault,
            0,
        );

        CFRelease(cf_key as *const core::ffi::c_void);
        IOObjectRelease(service);

        if cf_value.is_null() {
            return Err("HIDIdleTime property not found".to_string());
        }

        // Extract the i64 value (nanoseconds)
        let mut nanoseconds: i64 = 0;
        let success = CFNumberGetValue(
            cf_value as *const _,
            kCFNumberSInt64Type,
            &mut nanoseconds as *mut i64 as *mut core::ffi::c_void,
        );

        CFRelease(cf_value);

        if !success {
            return Err("Failed to extract CFNumber value".to_string());
        }

        Ok(nanoseconds as f64 / 1_000_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_idle_seconds_returns_reasonable_value() {
        // This test requires a running macOS display session
        match get_idle_seconds() {
            Ok(seconds) => {
                assert!(seconds >= 0.0, "idle time should be non-negative");
                // In CI or headless environments this might be very large,
                // but it should still be a finite number
                assert!(seconds.is_finite(), "idle time should be finite");
            }
            Err(e) => {
                // May fail in CI without a display session — that's OK
                eprintln!("IOKit idle time unavailable (expected in headless): {}", e);
            }
        }
    }
}
