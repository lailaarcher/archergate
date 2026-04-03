//! DNA Crypto Pipeline — export, import, encryption, signing, licensing.
//! Port of legacy/archergate-engine/src/dna-crypto.js
//!
//! Owns: .agdna file format, encryption, signing, machine fingerprint, license tokens.
//! Does NOT: decide when to export. That's the orchestrator's job.
//! Thread safety: all functions are pure (no shared state).
//!
//! CRITICAL: .agdna binary format must be byte-compatible with JS version.
//! Layout: [AGDNA1:6][salt:16][nonce:12][authTag:16][envLen:4][envelope:var][ciphertext:var]
//!
//! Constraint: raw session data never leaves the machine.
//! Only transition tables leave, encrypted and signed.

use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::Aead;
use aes_gcm::KeyInit as AesKeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::types::*;

type HmacSha256 = Hmac<Sha256>;

/// HKDF — extract-then-expand key derivation.
/// Matches the JS implementation exactly.
fn hkdf(ikm: &[u8], salt: &[u8], info: &str, length: usize) -> Vec<u8> {
    // Extract
    let mut mac = <HmacSha256 as Mac>::new_from_slice(salt)
        .expect("HMAC accepts any key length"); // safe: HMAC-SHA256 accepts any key size
    mac.update(ikm);
    let prk = mac.finalize().into_bytes();

    // Expand
    let mut t = Vec::new();
    let mut okm = Vec::new();
    let mut i: u8 = 1;

    while okm.len() < length {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&prk)
            .expect("HMAC accepts any key length"); // safe: same as above
        mac.update(&t);
        mac.update(info.as_bytes());
        mac.update(&[i]);
        t = mac.finalize().into_bytes().to_vec();
        okm.extend_from_slice(&t);
        i += 1;
    }

    okm.truncate(length);
    okm
}

/// Machine fingerprint: SHA256(hostname | platform | cpu_model | cpu_count).
pub fn machine_fingerprint() -> String {
    use sha2::Digest;
    let sys = sysinfo::System::new_all();
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".into());
    let platform = std::env::consts::OS;
    let cpu_model = sys.cpus().first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown".into());
    let cpu_count = sys.cpus().len();

    let raw = format!("{}|{}|{}|{}", hostname, platform, cpu_model, cpu_count);
    let hash = sha2::Sha256::digest(raw.as_bytes());
    hex::encode(hash)
}

/// Export a producer's models as an encrypted .agdna buffer.
pub fn export_dna(params: &ExportParams) -> Result<(Vec<u8>, DnaEnvelope)> {
    if params.master_secret.len() < 32 {
        return Err(ArchergateError::Crypto("masterSecret must be at least 32 bytes".into()));
    }

    // 1. Serialize transition tables
    let plaintext = serde_json::to_vec(&params.models)?;

    // 2. Derive encryption key
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt)
        .map_err(|e| ArchergateError::Crypto(format!("RNG failed: {}", e)))?;

    let ikm: Vec<u8> = [params.publisher_id.as_bytes(), &params.master_secret].concat();
    let encryption_key = hkdf(&ikm, &salt, "archergate-dna-export-v1", 32);

    // 3. Encrypt with AES-256-GCM
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes)
        .map_err(|e| ArchergateError::Crypto(format!("RNG failed: {}", e)))?;

    // Build envelope (used as AAD)
    let mut envelope = DnaEnvelope {
        publisher_id: params.publisher_id.clone(),
        producer_tag: params.producer_tag.clone(),
        model_version: params.models.version,
        session_count: params.models.meta.session_count,
        note_count: params.models.meta.total_events,
        genre_tags: params.genre_tags.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        preview_hash: None,
        dna_percent: params.models.meta.dna_percent,
        signature: None,
    };

    let envelope_json = serde_json::to_string(&envelope)?;

    // AES-256-GCM encrypt with envelope as AAD
    let cipher = <Aes256Gcm as AesKeyInit>::new_from_slice(&encryption_key)
        .map_err(|e| ArchergateError::Crypto(format!("cipher init: {}", e)))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    // aes-gcm crate appends the auth tag to the ciphertext
    let ciphertext_with_tag = cipher.encrypt(nonce, aes_gcm::aead::Payload {
        msg: &plaintext,
        aad: envelope_json.as_bytes(),
    }).map_err(|e| ArchergateError::Crypto(format!("encrypt: {}", e)))?;

    // Split: ciphertext is all but last 16 bytes, auth_tag is last 16
    let tag_start = ciphertext_with_tag.len() - 16;
    let ciphertext = &ciphertext_with_tag[..tag_start];
    let auth_tag = &ciphertext_with_tag[tag_start..];

    // 4. Sign the envelope
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&params.master_secret)
        .expect("HMAC accepts any key length"); // safe
    mac.update(envelope_json.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    envelope.signature = Some(format!("ARCHERGATE_SIG:{}", signature));

    // 5. Pack into .agdna format
    // Layout: [AGDNA1:6][salt:16][nonce:12][authTag:16][envLen:4][envelope:var][ciphertext:var]
    let envelope_with_sig = serde_json::to_vec(&envelope)?;
    let env_len = (envelope_with_sig.len() as u32).to_be_bytes();

    let mut agdna = Vec::new();
    agdna.extend_from_slice(b"AGDNA1");
    agdna.extend_from_slice(&salt);
    agdna.extend_from_slice(&nonce_bytes);
    agdna.extend_from_slice(auth_tag);
    agdna.extend_from_slice(&env_len);
    agdna.extend_from_slice(&envelope_with_sig);
    agdna.extend_from_slice(ciphertext);

    Ok((agdna, envelope))
}

/// Parse and verify a .agdna file. Does NOT decrypt.
pub fn verify_dna(agdna: &[u8], master_secret: &[u8]) -> VerifiedDna {
    // Magic bytes
    if agdna.len() < 54 || &agdna[0..6] != b"AGDNA1" {
        return VerifiedDna {
            is_valid: false,
            error: Some("Not a valid .agdna file".into()),
            envelope: empty_envelope(),
            salt: vec![], nonce: vec![], auth_tag: vec![], ciphertext: vec![],
        };
    }

    let salt = agdna[6..22].to_vec();
    let nonce = agdna[22..34].to_vec();
    let auth_tag = agdna[34..50].to_vec();
    let env_len = u32::from_be_bytes([agdna[50], agdna[51], agdna[52], agdna[53]]) as usize;

    if agdna.len() < 54 + env_len {
        return VerifiedDna {
            is_valid: false, error: Some("Truncated file".into()),
            envelope: empty_envelope(), salt, nonce, auth_tag, ciphertext: vec![],
        };
    }

    let envelope_bytes = &agdna[54..54 + env_len];
    let ciphertext = agdna[54 + env_len..].to_vec();

    let envelope: DnaEnvelope = match serde_json::from_slice(envelope_bytes) {
        Ok(e) => e,
        Err(e) => return VerifiedDna {
            is_valid: false, error: Some(format!("Bad envelope: {}", e)),
            envelope: empty_envelope(), salt, nonce, auth_tag, ciphertext,
        },
    };

    // Verify signature
    let sig = match &envelope.signature {
        Some(s) if s.starts_with("ARCHERGATE_SIG:") => s.clone(),
        _ => return VerifiedDna {
            is_valid: false, error: Some("Missing or invalid signature".into()),
            envelope, salt, nonce, auth_tag, ciphertext,
        },
    };

    // Re-derive signature from envelope without the signature field
    let mut env_for_signing = envelope.clone();
    env_for_signing.signature = None;
    let env_json = serde_json::to_string(&env_for_signing).unwrap_or_default();

    let mut mac = <HmacSha256 as Mac>::new_from_slice(master_secret)
        .expect("HMAC accepts any key length"); // safe
    mac.update(env_json.as_bytes());
    let expected_sig = format!("ARCHERGATE_SIG:{}", hex::encode(mac.finalize().into_bytes()));

    let is_valid = sig == expected_sig;

    VerifiedDna {
        is_valid,
        error: if is_valid { None } else { Some("Signature verification failed".into()) },
        envelope,
        salt, nonce, auth_tag, ciphertext,
    }
}

/// Decrypt a verified .agdna file.
pub fn decrypt_dna(verified: &VerifiedDna, publisher_id: &str, _machine_id: &str, master_secret: &[u8]) -> Result<ModelExport> {
    if !verified.is_valid {
        return Err(ArchergateError::InvalidDna("DNA file is not valid".into()));
    }

    // Derive the same encryption key the exporter used.
    // MVP: publisher key decrypts directly.
    let ikm: Vec<u8> = [publisher_id.as_bytes(), master_secret].concat();
    let publisher_key = hkdf(&ikm, &verified.salt, "archergate-dna-export-v1", 32);

    // Reconstruct AAD (envelope without signature)
    let mut env_for_aad = verified.envelope.clone();
    env_for_aad.signature = None;
    let aad = serde_json::to_string(&env_for_aad)?;

    // Reassemble ciphertext + auth_tag for aes-gcm
    let mut ct_with_tag = verified.ciphertext.clone();
    ct_with_tag.extend_from_slice(&verified.auth_tag);

    let cipher = <Aes256Gcm as AesKeyInit>::new_from_slice(&publisher_key)
        .map_err(|e| ArchergateError::Crypto(format!("cipher init: {}", e)))?;
    let nonce = Nonce::from_slice(&verified.nonce);

    let plaintext = cipher.decrypt(nonce, aes_gcm::aead::Payload {
        msg: &ct_with_tag,
        aad: aad.as_bytes(),
    }).map_err(|_| ArchergateError::Crypto("Decryption failed — wrong key or tampered data".into()))?;

    let models: ModelExport = serde_json::from_slice(&plaintext)?;
    Ok(models)
}

/// Generate a license token (simplified JWT for MVP).
pub fn generate_license_token(purchase_id: &str, machine_id: &str, secret: &[u8], expiry_days: u32) -> String {
    use base64::Engine as _;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0); // safe: clock always after epoch

    let payload = serde_json::json!({
        "pid": purchase_id,
        "mid": machine_id,
        "iat": now,
        "exp": now + (expiry_days as u64) * 24 * 60 * 60 * 1000,
    });

    let payload_str = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(payload.to_string().as_bytes());

    let mut mac = <HmacSha256 as Mac>::new_from_slice(secret)
        .expect("HMAC accepts any key length"); // safe
    mac.update(payload_str.as_bytes());
    let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(mac.finalize().into_bytes());

    format!("{}.{}", payload_str, sig)
}

/// Verify a license token.
pub fn verify_license_token(token: &str, machine_id: &str, secret: &[u8]) -> (bool, bool) {
    use base64::Engine as _;
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    if parts.len() != 2 { return (false, false); }

    let (payload_str, sig) = (parts[0], parts[1]);

    let mut mac = <HmacSha256 as Mac>::new_from_slice(secret)
        .expect("HMAC accepts any key length"); // safe
    mac.update(payload_str.as_bytes());
    let expected_sig = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(mac.finalize().into_bytes());

    if sig != expected_sig { return (false, false); }

    let payload_bytes = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload_str) {
        Ok(b) => b,
        Err(_) => return (false, false),
    };

    let payload: serde_json::Value = match serde_json::from_slice(&payload_bytes) {
        Ok(v) => v,
        Err(_) => return (false, false),
    };

    let mid = payload.get("mid").and_then(|v| v.as_str()).unwrap_or("");
    if mid != machine_id { return (false, false); }

    let exp = payload.get("exp").and_then(|v| v.as_u64()).unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if now > exp { return (false, true); } // valid sig but expired

    (true, false) // valid and not expired
}

fn empty_envelope() -> DnaEnvelope {
    DnaEnvelope {
        publisher_id: String::new(), producer_tag: String::new(),
        model_version: 0, session_count: 0, note_count: 0,
        genre_tags: vec![], created_at: String::new(),
        preview_hash: None, dna_percent: 0, signature: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram::PredictionEngine;
    use crate::types::DecisionVector;

    fn make_vec(note: u8) -> DecisionVector {
        DecisionVector {
            note, velocity: 100, beat_position: 0.0, duration_ms: 100, bpm: 140.0,
            key: 0, mode: 0, interval: 0, time_since_last_ms: 0, channel: 9,
            is_drum: true, session_minute: 0, hour_of_day: 14, looped_bar: false,
            was_deleted: false, timestamp_ms: 1000, session_id: "test".into(),
        }
    }

    fn make_secret() -> Vec<u8> {
        let mut secret = vec![0u8; 32];
        getrandom::getrandom(&mut secret).expect("RNG");
        secret
    }

    #[test]
    fn exports_and_verifies() {
        let mut engine = PredictionEngine::new(3, 0.95);
        for i in 0..100u8 { engine.observe(&make_vec(36 + (i % 12))); }

        let secret = make_secret();
        let params = ExportParams {
            models: engine.export_models(),
            publisher_id: "prod_test123".into(),
            producer_tag: "TestProducer".into(),
            genre_tags: vec!["trap".into(), "drill".into()],
            master_secret: secret.clone(),
        };

        let (agdna, envelope) = export_dna(&params).expect("export");
        assert!(!agdna.is_empty());
        assert_eq!(envelope.publisher_id, "prod_test123");

        let verified = verify_dna(&agdna, &secret);
        assert!(verified.is_valid, "Signature should be valid");
    }

    #[test]
    fn full_round_trip() {
        let mut engine = PredictionEngine::new(3, 0.95);
        for i in 0..100u8 { engine.observe(&make_vec(36 + (i % 12))); }

        let secret = make_secret();
        let (agdna, _) = export_dna(&ExportParams {
            models: engine.export_models(),
            publisher_id: "prod_rt".into(),
            producer_tag: "RoundTrip".into(),
            genre_tags: vec!["house".into()],
            master_secret: secret.clone(),
        }).expect("export");

        let verified = verify_dna(&agdna, &secret);
        assert!(verified.is_valid);

        let decrypted = decrypt_dna(&verified, "prod_rt", &machine_fingerprint(), &secret)
            .expect("decrypt");
        assert!(!decrypted.harmony.is_empty());
        assert!(!decrypted.rhythm.is_empty());

        let mut engine2 = PredictionEngine::default();
        engine2.import_models(&decrypted);
        assert!(engine2.harmony.context_count() > 0);
    }

    #[test]
    fn rejects_tampered_dna() {
        let mut engine = PredictionEngine::new(3, 0.95);
        for i in 0..50u8 { engine.observe(&make_vec(60)); }

        let secret = make_secret();
        let (mut agdna, _) = export_dna(&ExportParams {
            models: engine.export_models(),
            publisher_id: "prod_tamper".into(),
            producer_tag: "TamperTest".into(),
            genre_tags: vec![],
            master_secret: secret.clone(),
        }).expect("export");

        // Tamper with the data
        let last = agdna.len() - 10;
        agdna[last] ^= 0xFF;

        let verified = verify_dna(&agdna, &secret);
        if verified.is_valid {
            // If signature still valid (tampered ciphertext, not envelope),
            // decrypt should fail
            assert!(decrypt_dna(&verified, "prod_tamper", &machine_fingerprint(), &secret).is_err());
        }
    }

    #[test]
    fn machine_fingerprint_is_deterministic() {
        assert_eq!(machine_fingerprint(), machine_fingerprint());
        assert_eq!(machine_fingerprint().len(), 64);
    }

    #[test]
    fn license_token_round_trip() {
        let secret = make_secret();
        let mid = machine_fingerprint();
        let token = generate_license_token("purchase_1", &mid, &secret, 30);

        let (valid, expired) = verify_license_token(&token, &mid, &secret);
        assert!(valid);
        assert!(!expired);

        // Wrong machine
        let (valid2, _) = verify_license_token(&token, "wrong_machine", &secret);
        assert!(!valid2);
    }
}
