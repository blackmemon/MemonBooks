// commands.rs — Tauri commands for device fingerprinting (Android + iOS)
// Desktop fingerprinting is done in Python (device_fingerprint.py).
// These commands are invoked from JS via Tauri's invoke() on mobile.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub platform: String,
    pub android_id: Option<String>,
    pub imei_1: Option<String>,
    pub imei_2: Option<String>,
    pub mac_address: Option<String>,
    pub idfv: Option<String>,
    pub hw_machine: Option<String>,
    pub device_hash: String,
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{:02x}", b);
        acc
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Android
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
fn get_android_id() -> Option<String> {
    use jni::JNIEnv;
    use jni::objects::{JObject, JValue};

    // Tauri v2 exposes the JNI env via tauri::api::private
    // In production, we get env via the Tauri Android plugin bridge.
    // The actual JNI call requires tauri-plugin-os to be enabled.
    // Placeholder: real ANDROID_ID fetch via Settings.Secure.
    None // Replaced at runtime via JS plugin call (see frontend security.js)
}

#[cfg(target_os = "android")]
#[tauri::command]
pub fn get_android_device_info() -> Result<DeviceInfo, String> {
    // On Android, privileged data (IMEI) requires READ_PHONE_STATE permission.
    // IMEI is read via tauri-plugin + Java bridge.
    // For devices running Android 10+, IMEI access requires carrier privilege.
    // ANDROID_ID is stable per app + signing key, reset only on factory reset.

    let android_id = std::env::var("MEMON_ANDROID_ID").ok();
    let imei_1 = std::env::var("MEMON_IMEI_1").ok();
    let imei_2 = std::env::var("MEMON_IMEI_2").ok();
    let mac_address = std::env::var("MEMON_MAC_ADDRESS").ok();

    let raw = format!(
        "android|{}|{}|{}|{}",
        android_id.as_deref().unwrap_or("UNKNOWN"),
        imei_1.as_deref().unwrap_or("UNKNOWN"),
        imei_2.as_deref().unwrap_or("UNKNOWN"),
        mac_address.as_deref().unwrap_or("UNKNOWN"),
    );

    let device_hash = sha256_hex(&raw);

    Ok(DeviceInfo {
        platform: "android".to_string(),
        android_id,
        imei_1,
        imei_2,
        mac_address,
        idfv: None,
        hw_machine: None,
        device_hash,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// iOS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "ios")]
#[tauri::command]
pub fn get_ios_device_info() -> Result<DeviceInfo, String> {
    // IDFV (Identifier for Vendor) — stable per vendor, reset on full uninstall.
    // HW Machine — e.g. "iPhone15,2" — from uname.machine via sysctlbyname.
    // IMEI not accessible on iOS without entitlement (carriers only).
    // MAC address always returns 02:00:00:00:00:00 on iOS (privacy restriction).

    let idfv = get_idfv_ios();
    let hw_machine = get_hw_machine_ios();

    let raw = format!(
        "ios|{}|{}",
        idfv.as_deref().unwrap_or("UNKNOWN"),
        hw_machine.as_deref().unwrap_or("UNKNOWN"),
    );

    let device_hash = sha256_hex(&raw);

    Ok(DeviceInfo {
        platform: "ios".to_string(),
        android_id: None,
        imei_1: None,
        imei_2: None,
        mac_address: None,
        idfv,
        hw_machine,
        device_hash,
    })
}

#[cfg(target_os = "ios")]
fn get_idfv_ios() -> Option<String> {
    // Swift bridge via Tauri iOS plugin returns IDFV as env var set before Rust init.
    std::env::var("MEMON_IDFV").ok()
}

#[cfg(target_os = "ios")]
fn get_hw_machine_ios() -> Option<String> {
    use std::ffi::CString;
    use std::mem;

    unsafe {
        let name = CString::new("hw.machine").ok()?;
        let mut size: libc::size_t = 0;
        if libc::sysctlbyname(name.as_ptr(), std::ptr::null_mut(), &mut size, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        let mut buf = vec![0u8; size];
        if libc::sysctlbyname(name.as_ptr(), buf.as_mut_ptr() as *mut libc::c_void, &mut size, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        // Remove trailing null byte
        buf.retain(|&b| b != 0);
        String::from_utf8(buf).ok()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Desktop stub (used when building for desktop — prevents compile error)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub fn get_android_device_info() -> Result<DeviceInfo, String> {
    Err("get_android_device_info is only available on Android.".to_string())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub fn get_ios_device_info() -> Result<DeviceInfo, String> {
    Err("get_ios_device_info is only available on iOS.".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Desktop device hash — delegates to Python backend
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_desktop_device_hash(backend_url: String) -> Result<String, String> {
    // The Python backend computes the desktop fingerprint and returns the hash.
    // Called from auth.js during login on desktop.
    let url = format!("{}/api/device/hash", backend_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Backend returned {}", resp.status()));
    }

    #[derive(Deserialize)]
    struct HashResponse {
        device_hash: String,
    }

    let body: HashResponse = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {e}"))?;

    Ok(body.device_hash)
}

// ─────────────────────────────────────────────────────────────────────────────
// Register all commands with Tauri builder (called from main.rs)
// ─────────────────────────────────────────────────────────────────────────────

pub fn register_handlers() -> impl Fn(tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    |builder| {
        builder.invoke_handler(tauri::generate_handler![
            get_android_device_info,
            get_ios_device_info,
            get_desktop_device_hash,
        ])
    }
}
