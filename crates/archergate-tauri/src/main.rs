//! ARCHERGATE Desktop App — Tauri v2 entry point.
//!
//! Owns: app lifecycle, window creation, IPC server startup.
//! Does NOT: implement engine logic (that's archergate-core).
//! Tauri: v2 — uses tauri::Builder, not v1 patterns.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;

use std::sync::Arc;
use tokio::sync::Mutex;
use ipc::server::{EngineHandle, SharedEngine};

fn main() {
    let engine: SharedEngine = Arc::new(Mutex::new(EngineHandle::new()));

    tauri::Builder::default()
        .manage(engine.clone())
        .setup(move |_app| {
            // Start the IPC WebSocket server on 127.0.0.1:39741
            ipc::server::start(engine);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running archergate"); // justified: app cannot recover from failed launch
}
