//! IPC WebSocket Server — localhost:39741.
//!
//! Owns: accepting connections, parsing messages, routing to engine, responding.
//! Does NOT: own the engine or storage. Holds an Arc<Mutex<Engine>> reference.
//! Thread safety: runs as a tokio task. Engine lock held only during observe+predict (<1ms).
//!   SessionStore write is spawned separately, never awaited inline.
//! Tauri: v2 — started in setup(), shares state with Tauri commands.
//!
//! Latency budget:
//!   observe + predict < 1ms (lock held)
//!   SessionStore write: spawned task, not on critical path
//!   Total round-trip target: < 3ms

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn, error};

use crate::ipc::types::*;

/// Shared engine state — wrapped for thread-safe access.
/// The Mutex is tokio::sync::Mutex so it's safe across await points.
pub type SharedEngine = Arc<Mutex<EngineHandle>>;

/// Holds the engine + storage references the server needs.
/// This is the single struct behind the Arc<Mutex<>>.
pub struct EngineHandle {
    pub engine: archergate_core::ngram::PredictionEngine,
    pub session_state: archergate_core::decision_vector::SessionState,
    pub current_session_id: Option<String>,
    // TODO: add Storage and blend state when those modules are ported
}

impl EngineHandle {
    pub fn new() -> Self {
        Self {
            engine: archergate_core::ngram::PredictionEngine::new(3, 0.95),
            session_state: archergate_core::decision_vector::SessionState::new(),
            current_session_id: None,
        }
    }
}

const PORT: u16 = 39741;

/// Start the IPC server. Call from Tauri setup().
/// Returns immediately — the server runs as a background tokio task.
pub fn start(engine: SharedEngine) {
    tokio::spawn(async move {
        if let Err(e) = run_server(engine).await {
            error!("IPC server fatal: {}", e);
        }
    });
}

async fn run_server(engine: SharedEngine) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("127.0.0.1:{}", PORT);
    let listener = TcpListener::bind(&addr).await?;
    info!("ARCHERGATE engine on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                info!("Plugin connected from {}", peer);
                let engine = engine.clone();

                tokio::spawn(async move {
                    match accept_async(stream).await {
                        Ok(ws_stream) => {
                            if let Err(e) = handle_connection(ws_stream, engine).await {
                                warn!("Connection closed: {}", e);
                            }
                        }
                        Err(e) => warn!("WebSocket handshake failed: {}", e),
                    }
                });
            }
            Err(e) => {
                warn!("Accept failed: {}", e);
            }
        }
    }
}

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    engine: SharedEngine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut sink, mut stream) = ws_stream.split();

    // Send init message
    {
        let handle = engine.lock().await;
        let init = OutgoingMessage::Init(InitPayload {
            bpm: 120.0,
            dna: DnaStatus {
                percent: handle.engine.dna_percent(),
                stage: handle.engine.dna_stage(),
                sessions: handle.engine.session_count,
            },
            total_transitions: handle.engine.harmony.context_count(),
        });
        let json = serde_json::to_string(&init)?;
        sink.send(tokio_tungstenite::tungstenite::Message::Text(json)).await?;
    }

    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!("Read error: {}", e);
                break;
            }
        };

        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            tokio_tungstenite::tungstenite::Message::Close(_) => break,
            tokio_tungstenite::tungstenite::Message::Ping(d) => {
                let _ = sink.send(tokio_tungstenite::tungstenite::Message::Pong(d)).await;
                continue;
            }
            _ => continue,
        };

        let incoming: IncomingMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                warn!("Bad JSON: {}", e);
                let err = OutgoingMessage::Error { error: format!("Invalid JSON: {}", e) };
                let _ = sink.send(tokio_tungstenite::tungstenite::Message::Text(
                    serde_json::to_string(&err).unwrap_or_default() // safe: Error is always serializable
                )).await;
                continue;
            }
        };

        let response = handle_message(incoming, &engine).await;

        let json = serde_json::to_string(&response).unwrap_or_default(); // safe: OutgoingMessage is always serializable
        if sink.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.is_err() {
            break;
        }
    }

    info!("Plugin disconnected");
    Ok(())
}

/// Route an incoming message to the engine. Lock is held only for observe+predict.
async fn handle_message(msg: IncomingMessage, engine: &SharedEngine) -> OutgoingMessage {
    match msg {
        IncomingMessage::Note(event) => handle_note(event, engine).await,

        IncomingMessage::Detection(det) => {
            let mut handle = engine.lock().await;
            handle.session_state.update_state(det.bpm, det.key, det.mode);
            OutgoingMessage::Detection { ok: true }
        }

        IncomingMessage::SessionStart { genre: _ } => {
            let mut handle = engine.lock().await;
            let session_id = uuid::Uuid::new_v4().to_string();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0); // safe: system clock is always after epoch
            handle.session_state.start_session(session_id.clone(), now);
            handle.engine.new_session();
            handle.current_session_id = Some(session_id.clone());
            OutgoingMessage::SessionStart { session_id }
        }

        IncomingMessage::SessionEnd => {
            let mut handle = engine.lock().await;
            match handle.current_session_id.take() {
                Some(id) => OutgoingMessage::SessionEnd(SessionEndPayload {
                    session_id: id,
                    analytics: serde_json::json!({}), // TODO: wire session analytics
                }),
                None => OutgoingMessage::Error { error: "No active session".into() },
            }
        }

        IncomingMessage::Predict => {
            let handle = engine.lock().await;
            let preds = handle.engine.predict().unwrap_or_default();
            OutgoingMessage::Predict {
                predictions: preds.iter().map(|p| PredictionItem {
                    note: p.note,
                    probability: p.probability,
                    velocity: p.velocity,
                    beat_position: p.beat_position,
                }).collect(),
            }
        }

        IncomingMessage::Summary => {
            let handle = engine.lock().await;
            let summary = handle.engine.summary();
            OutgoingMessage::Summary(
                serde_json::to_value(&summary).unwrap_or_default() // safe: EngineSummary is Serialize
            )
        }

        IncomingMessage::Reset => {
            let mut handle = engine.lock().await;
            handle.engine = archergate_core::ngram::PredictionEngine::new(3, 0.95);
            OutgoingMessage::Reset { ok: true }
        }

        // TODO: wire these when storage + crypto modules are ported
        IncomingMessage::DnaProfile => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::SessionAnalytics { .. } => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::ExportDna { .. } => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::ImportDna { .. } => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::SetBlend { .. } => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::ClearBlend => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::ListModels => OutgoingMessage::Error { error: "Not yet implemented".into() },
        IncomingMessage::Provenance { .. } => OutgoingMessage::Error { error: "Not yet implemented".into() },
    }
}

/// Handle a note event. This is the hot path — latency budget: < 3ms total.
async fn handle_note(event: NoteEvent, engine: &SharedEngine) -> OutgoingMessage {
    let t0 = std::time::Instant::now();

    let now = event.timestamp_ms.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0) // safe: system clock is always after epoch
    });

    let raw = archergate_core::types::RawMidiEvent {
        note: event.note,
        velocity: event.velocity,
        channel: event.channel,
        timestamp_ms: now,
    };

    let ctx = archergate_core::types::EventContext {
        bpm: event.bpm,
        key: event.key,
        mode: event.mode,
        looped_bar: event.looped_bar,
        was_deleted: event.was_deleted,
        duration_ms: event.duration_ms,
    };

    // Lock held only for observe + predict (< 1ms)
    let response = {
        let mut handle = engine.lock().await;

        // Auto-start session if none active
        if handle.current_session_id.is_none() {
            let session_id = uuid::Uuid::new_v4().to_string();
            handle.session_state.start_session(session_id.clone(), now);
            handle.engine.new_session();
            handle.current_session_id = Some(session_id);
        }

        let vec = handle.session_state.vectorize(&raw, &ctx);
        handle.engine.observe(&vec);

        let preds = handle.engine.predict().unwrap_or_default();
        let latency_ms = t0.elapsed().as_secs_f64() * 1000.0;

        PredictionResponse {
            predictions: preds.iter().map(|p| PredictionItem {
                note: p.note,
                probability: p.probability,
                velocity: p.velocity,
                beat_position: p.beat_position,
            }).collect(),
            bpm: vec.bpm,
            key: vec.key,
            mode: vec.mode,
            dna_percent: handle.engine.dna_percent(),
            dna_stage: handle.engine.dna_stage(),
            latency_ms: (latency_ms * 100.0).round() / 100.0,
            session_minute: vec.session_minute,
        }
    };
    // Lock released here — SessionStore write would be spawned after this point

    OutgoingMessage::Note(response)
}
