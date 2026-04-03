/*
 * archergate_license.h — C API for the Archergate License SDK
 *
 * Link against:
 *   Windows: archergate_license.dll.lib  (dynamic) or archergate_license.lib (static)
 *   macOS:   libarchergate_license.dylib (dynamic) or libarchergate_license.a  (static)
 *   Linux:   libarchergate_license.so    (dynamic) or libarchergate_license.a  (static)
 *
 * Thread safety: all functions are safe to call from any thread.
 * The client handle must be freed with ag_license_free().
 */

#ifndef ARCHERGATE_LICENSE_H
#define ARCHERGATE_LICENSE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Error codes ─────────────────────────────────────────────────── */

#define AG_OK                     0
#define AG_ERR_INVALID           -1
#define AG_ERR_EXPIRED           -2
#define AG_ERR_MACHINE_MISMATCH  -3
#define AG_ERR_NETWORK           -4
#define AG_ERR_TRIAL_EXPIRED     -5
#define AG_ERR_ACTIVATION_LIMIT  -6

/* ── Opaque handle ───────────────────────────────────────────────── */

typedef struct AgLicenseClient AgLicenseClient;

/* ── Lifecycle ───────────────────────────────────────────────────── */

/*
 * Create a new license client.
 * Returns NULL if api_key or plugin_id is NULL / invalid UTF-8.
 */
AgLicenseClient* ag_license_new(const char* api_key, const char* plugin_id);

/*
 * Create a license client with a custom server URL.
 * Use this for self-hosted license servers.
 */
AgLicenseClient* ag_license_new_with_url(
    const char* api_key,
    const char* plugin_id,
    const char* api_url
);

/*
 * Free a license client. Safe to call with NULL.
 * Do NOT use the pointer after calling this.
 */
void ag_license_free(AgLicenseClient* client);

/* ── License operations ──────────────────────────────────────────── */

/*
 * Validate a license key.
 * Returns AG_OK (0) on success, or a negative AG_ERR_* code.
 *
 * Checks local cache first. Falls back to the API.
 * Works offline for 30 days after last successful validation.
 */
int ag_license_validate(const AgLicenseClient* client, const char* license_key);

/*
 * Activate a license on this machine.
 * Call once when the user first enters their key.
 * Returns AG_OK (0) on success, or a negative AG_ERR_* code.
 */
int ag_license_activate(
    const AgLicenseClient* client,
    const char* license_key,
    const char* email
);

/*
 * Start a 14-day trial. No server call needed.
 * Returns AG_OK (0) on success, AG_ERR_TRIAL_EXPIRED if already used.
 * Writes remaining days to *out_days_remaining (if not NULL).
 */
int ag_license_start_trial(
    const AgLicenseClient* client,
    uint32_t* out_days_remaining
);

/* ── Utilities ───────────────────────────────────────────────────── */

/*
 * Write the machine fingerprint (64 hex chars + null) into buf.
 * buf must be at least 65 bytes. Returns 0 on success, -1 on error.
 */
int ag_license_fingerprint(char* buf, unsigned long buf_len);

/*
 * Get a human-readable error message for an error code.
 * Always returns a valid static string (never NULL).
 */
const char* ag_license_error_string(int code);

#ifdef __cplusplus
} /* extern "C" */
#endif

/* ── C++ convenience wrapper ─────────────────────────────────────── */

#ifdef __cplusplus
#include <memory>
#include <string>
#include <stdexcept>

namespace archergate {

class LicenseException : public std::runtime_error {
public:
    int code;
    LicenseException(int c)
        : std::runtime_error(ag_license_error_string(c)), code(c) {}
};

class License {
public:
    License(const char* apiKey, const char* pluginId)
        : client_(ag_license_new(apiKey, pluginId), &ag_license_free) {
        if (!client_) throw std::runtime_error("Failed to create license client");
    }

    License(const char* apiKey, const char* pluginId, const char* apiUrl)
        : client_(ag_license_new_with_url(apiKey, pluginId, apiUrl), &ag_license_free) {
        if (!client_) throw std::runtime_error("Failed to create license client");
    }

    void validate(const char* licenseKey) {
        int rc = ag_license_validate(client_.get(), licenseKey);
        if (rc != AG_OK) throw LicenseException(rc);
    }

    void activate(const char* licenseKey, const char* email) {
        int rc = ag_license_activate(client_.get(), licenseKey, email);
        if (rc != AG_OK) throw LicenseException(rc);
    }

    uint32_t startTrial() {
        uint32_t days = 0;
        int rc = ag_license_start_trial(client_.get(), &days);
        if (rc != AG_OK) throw LicenseException(rc);
        return days;
    }

    static std::string fingerprint() {
        char buf[65];
        ag_license_fingerprint(buf, sizeof(buf));
        return std::string(buf, 64);
    }

private:
    std::unique_ptr<AgLicenseClient, decltype(&ag_license_free)> client_;
};

} /* namespace archergate */
#endif

#endif /* ARCHERGATE_LICENSE_H */
