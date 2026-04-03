//! Machine fingerprint — a stable, per-machine identifier.
//!
//! SHA-256 of (CPU brand string + OS install ID).
//! Stable across reboots. Changes only when hardware changes.

use sha2::{Sha256, Digest};

/// Returns a lowercase hex SHA-256 fingerprint for this machine.
///
/// Combines the CPU brand string with a platform-specific machine ID:
/// - Linux: `/etc/machine-id`
/// - macOS: `IOPlatformUUID` via `ioreg`
/// - Windows: `MachineGuid` from the registry
pub fn machine_fingerprint() -> String {
    let cpu_brand = cpu_brand_string();
    let os_id = os_install_id();
    let raw = format!("{cpu_brand}|{os_id}");
    let hash = Sha256::digest(raw.as_bytes());
    hex::encode(hash)
}

fn cpu_brand_string() -> String {
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything()),
    );
    sys.cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown-cpu".into())
}

#[cfg(target_os = "linux")]
fn os_install_id() -> String {
    std::fs::read_to_string("/etc/machine-id")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown-linux-id".into())
}

#[cfg(target_os = "macos")]
fn os_install_id() -> String {
    std::process::Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()
        .and_then(|out| {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.contains("IOPlatformUUID") {
                    // Line looks like: "IOPlatformUUID" = "XXXXXXXX-..."
                    if let Some(uuid) = line.split('"').nth(3) {
                        return Some(uuid.to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| "unknown-macos-id".into())
}

#[cfg(target_os = "windows")]
fn os_install_id() -> String {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Cryptography")
        .and_then(|key| key.get_value::<String, _>("MachineGuid"))
        .unwrap_or_else(|_| "unknown-windows-id".into())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn os_install_id() -> String {
    "unknown-platform-id".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic() {
        let a = machine_fingerprint();
        let b = machine_fingerprint();
        assert_eq!(a, b, "fingerprint must be stable across calls");
    }

    #[test]
    fn fingerprint_is_64_hex_chars() {
        let fp = machine_fingerprint();
        assert_eq!(fp.len(), 64, "SHA-256 hex = 64 chars");
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()), "must be valid hex");
    }

    #[test]
    fn fingerprint_is_lowercase() {
        let fp = machine_fingerprint();
        assert_eq!(fp, fp.to_lowercase());
    }
}
