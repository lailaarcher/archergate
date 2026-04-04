use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};

use archergate_license::LicenseClient;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("License validation failed: {0}")]
    Validation(String),
    #[error("Activation failed: {0}")]
    Activation(String),
    #[error("Trial check failed: {0}")]
    Trial(String),
    #[error("Plugin state error: {0}")]
    State(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// Types returned to the frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub message: String,
    /// When the license expires, as a Unix timestamp (seconds). None for
    /// perpetual licenses.
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialStatus {
    pub active: bool,
    pub days_remaining: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub licensed: bool,
    pub trial_active: bool,
    pub trial_days_remaining: i32,
    pub license_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationResult {
    pub success: bool,
    pub message: String,
    pub machine_locked: bool,
}

// ---------------------------------------------------------------------------
// Plugin state -- shared across all Tauri commands
// ---------------------------------------------------------------------------

struct PluginState {
    client: LicenseClient,
    /// Cached license key after a successful activation or validation.
    active_key: Mutex<Option<String>>,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn validate_license(
    state: State<'_, PluginState>,
    key: String,
) -> Result<ValidationResult> {
    match state.client.validate(&key) {
        Ok(()) => {
            let mut active_key = state
                .active_key
                .lock()
                .map_err(|e| Error::State(e.to_string()))?;
            *active_key = Some(key);
            Ok(ValidationResult {
                valid: true,
                message: "License is valid.".into(),
                expires_at: None,
            })
        }
        Err(e) => Ok(ValidationResult {
            valid: false,
            message: format!("Invalid license: {e}"),
            expires_at: None,
        }),
    }
}

#[tauri::command]
async fn check_trial(state: State<'_, PluginState>) -> Result<TrialStatus> {
    let days = state.client.trial_days_remaining();
    Ok(TrialStatus {
        active: days > 0,
        days_remaining: days,
    })
}

#[tauri::command]
async fn get_license_status(state: State<'_, PluginState>) -> Result<LicenseStatus> {
    let active_key = state
        .active_key
        .lock()
        .map_err(|e| Error::State(e.to_string()))?;

    let licensed = active_key.is_some();
    let trial_days = state.client.trial_days_remaining();

    Ok(LicenseStatus {
        licensed,
        trial_active: trial_days > 0,
        trial_days_remaining: trial_days,
        license_key: active_key.clone(),
    })
}

#[tauri::command]
async fn activate_license(
    state: State<'_, PluginState>,
    key: String,
) -> Result<ActivationResult> {
    match state.client.validate(&key) {
        Ok(()) => {
            let mut active_key = state
                .active_key
                .lock()
                .map_err(|e| Error::State(e.to_string()))?;
            *active_key = Some(key);
            Ok(ActivationResult {
                success: true,
                message: "License activated and locked to this machine.".into(),
                machine_locked: true,
            })
        }
        Err(e) => Ok(ActivationResult {
            success: false,
            message: format!("Activation failed: {e}"),
            machine_locked: false,
        }),
    }
}

// ---------------------------------------------------------------------------
// Builder -- public API used by Tauri app developers
// ---------------------------------------------------------------------------

/// Configuration passed to `archergate::init()`.
pub struct ArchergateConfig {
    pub api_key: String,
    pub app_id: String,
}

/// Initialize the Archergate licensing plugin.
///
/// Call this in your Tauri `main.rs`:
///
/// ```rust,no_run
/// fn main() {
///     tauri::Builder::default()
///         .plugin(tauri_plugin_archergate::init("your-api-key", "com.you.app"))
///         .run(tauri::generate_context!())
///         .expect("error while running tauri application");
/// }
/// ```
pub fn init<R: Runtime>(api_key: &str, app_id: &str) -> TauriPlugin<R> {
    let config = ArchergateConfig {
        api_key: api_key.to_string(),
        app_id: app_id.to_string(),
    };

    Builder::new("archergate")
        .invoke_handler(tauri::generate_handler![
            validate_license,
            check_trial,
            get_license_status,
            activate_license,
        ])
        .setup(move |app, _api| {
            let client = LicenseClient::new(&config.api_key, &config.app_id);
            app.manage(PluginState {
                client,
                active_key: Mutex::new(None),
            });
            Ok(())
        })
        .build()
}
