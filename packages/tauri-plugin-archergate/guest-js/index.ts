import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ValidationResult {
  valid: boolean;
  message: string;
  /** Unix timestamp (seconds) when the license expires. Null for perpetual. */
  expires_at: number | null;
}

export interface TrialStatus {
  active: boolean;
  days_remaining: number;
}

export interface LicenseStatus {
  licensed: boolean;
  trial_active: boolean;
  trial_days_remaining: number;
  license_key: string | null;
}

export interface ActivationResult {
  success: boolean;
  message: string;
  machine_locked: boolean;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/**
 * Validate a license key against the Archergate server.
 * Does not persist the key -- use `activateLicense` for that.
 */
export async function validateLicense(key: string): Promise<ValidationResult> {
  return invoke<ValidationResult>("plugin:archergate|validate_license", {
    key,
  });
}

/**
 * Check the current trial status for this machine.
 * Returns the number of days remaining and whether the trial is still active.
 */
export async function checkTrial(): Promise<TrialStatus> {
  return invoke<TrialStatus>("plugin:archergate|check_trial");
}

/**
 * Get the full license status: whether a license is active, trial state,
 * and the current key (if any).
 */
export async function getLicenseStatus(): Promise<LicenseStatus> {
  return invoke<LicenseStatus>("plugin:archergate|get_license_status");
}

/**
 * Activate a license key. Validates against the server and locks it to
 * this machine. On success the key is stored in plugin state for the
 * lifetime of the application.
 */
export async function activateLicense(key: string): Promise<ActivationResult> {
  return invoke<ActivationResult>("plugin:archergate|activate_license", {
    key,
  });
}
