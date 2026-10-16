#![forbid(unsafe_code)]
mod engine;
mod protocol;

use engine::Engine;
use photocraft_plugin_api::{
    ApiError, ErrorCode, HostWorkerFrame, MAX_WORKER_FRAME_BYTES, WorkerHostFrame, WorkerOperation,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const PLUGIN_ID: &str = "com.photocraft.bria-rmbg";
type Result<T> = std::result::Result<T, String>;
type Writer = Arc<Mutex<std::io::Stdout>>;

fn read_frame(reader: &mut impl Read) -> Result<HostWorkerFrame> {
    let mut header = [0u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|e| format!("Host input closed: {e}"))?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_WORKER_FRAME_BYTES {
        return Err("Invalid frame length.".into());
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    // Serde's internally tagged unit variant otherwise ignores extra shutdown fields.
    if value.get("type").and_then(serde_json::Value::as_str) == Some("shutdown")
        && value.as_object().is_none_or(|object| object.len() != 1)
    {
        return Err("Unexpected shutdown fields.".into());
    }
    let frame: HostWorkerFrame = serde_json::from_value(value).map_err(|e| e.to_string())?;
    frame.validate().map_err(|e| e.to_string())?;
    Ok(frame)
}

fn send(writer: &Writer, frame: WorkerHostFrame) -> Result<()> {
    frame.validate().map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&frame).map_err(|e| e.to_string())?;
    let mut writer = writer.lock().map_err(|_| "Output lock failed.")?;
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|_| writer.write_all(&bytes))
        .and_then(|_| writer.flush())
        .map_err(|e| e.to_string())
}

struct Active {
    id: String,
    cancel: Arc<AtomicBool>,
    thread: thread::JoinHandle<(Engine, Result<()>)>,
}

fn serve(runtime: PathBuf) -> Result<()> {
    let (tx, rx) = mpsc::sync_channel(8);
    // A full input queue is a protocol flood. Exit immediately, never accumulate frames.
    thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        loop {
            let frame = read_frame(&mut input);
            let failed = frame.is_err();
            if tx.try_send(frame).is_err() {
                std::process::exit(2);
            }
            if failed {
                break;
            }
        }
    });
    let hello = rx
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| "Host handshake timed out.")??;
    let writer: Writer = Arc::new(Mutex::new(std::io::stdout()));
    match hello {
        HostWorkerFrame::Hello {
            id,
            protocol_version,
            plugin_id,
            generation,
            operations,
            ..
        } if plugin_id == PLUGIN_ID && operations == [WorkerOperation::Image] => {
            send(
                &writer,
                WorkerHostFrame::Ready {
                    id,
                    protocol_version,
                    plugin_id,
                    generation,
                    operations,
                },
            )?;
        }
        _ => return Err("BRIA worker requires its own image-only protocol 2 handshake.".into()),
    }
    let mut engine = Engine::default();
    let mut active: Option<Active> = None;
    let mut previous_id = 0u64;
    loop {
        if active.as_ref().is_some_and(|job| job.thread.is_finished()) {
            let completed = active.take().ok_or("Missing completed job.")?;
            let (returned, result) = completed
                .thread
                .join()
                .map_err(|_| "BRIA worker thread failed.")?;
            engine = returned;
            let frame = if completed.cancel.load(Ordering::Acquire) {
                WorkerHostFrame::Cancelled { id: completed.id }
            } else {
                match result {
                    Ok(()) => WorkerHostFrame::Result { id: completed.id },
                    Err(message) => WorkerHostFrame::Error {
                        id: completed.id,
                        error: ApiError::new(
                            ErrorCode::InvalidJson,
                            message.chars().take(2000).collect::<String>(),
                        ),
                    },
                }
            };
            send(&writer, frame)?;
        }
        let frame = match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => {
                if let Some(job) = &active {
                    job.cancel.store(true, Ordering::Release);
                }
                return Err(error);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => return Err("Host input closed.".into()),
        };
        match frame {
            HostWorkerFrame::Shutdown => {
                if let Some(job) = active.take() {
                    job.cancel.store(true, Ordering::Release);
                    let _ = job.thread.join();
                }
                return Ok(());
            }
            HostWorkerFrame::Cancel { id } => {
                let job = active
                    .as_ref()
                    .filter(|job| job.id == id)
                    .ok_or("Cancellation ID does not match an active job.")?;
                job.cancel.store(true, Ordering::Release);
            }
            HostWorkerFrame::Run {
                id,
                operation,
                request,
                ..
            } => {
                let numeric = id.parse::<u64>().map_err(|e| e.to_string())?;
                if active.is_some() || numeric <= previous_id || operation != WorkerOperation::Image
                {
                    return Err("Unexpected run ID, operation or overlapping request.".into());
                }
                previous_id = numeric;
                let cancel = Arc::new(AtomicBool::new(false));
                let flag = cancel.clone();
                let output = writer.clone();
                let request_id = id.clone();
                let runtime = runtime.clone();
                let mut owned = std::mem::take(&mut engine);
                let thread = thread::spawn(move || {
                    let result = protocol::Job::parse(request).and_then(|job| {
                        owned.process(&job, &runtime, &flag, &|fraction, message| {
                            if send(
                                &output,
                                WorkerHostFrame::Progress {
                                    id: request_id.clone(),
                                    fraction,
                                    message: message.into(),
                                },
                            )
                            .is_err()
                            {
                                flag.store(true, Ordering::Release);
                            }
                        })
                    });
                    (owned, result)
                });
                active = Some(Active { id, cancel, thread });
            }
            HostWorkerFrame::Reset { id } => {
                let numeric = id.parse::<u64>().map_err(|e| e.to_string())?;
                if active.is_some() || numeric <= previous_id {
                    return Err("Unexpected reset ID or active request.".into());
                }
                previous_id = numeric;
                engine = Engine::default();
                send(&writer, WorkerHostFrame::ResetComplete { id })?;
            }
            _ => return Err("Unexpected host frame.".into()),
        }
    }
}

fn main() {
    let result = (|| {
        if std::env::args().skip(1).collect::<Vec<_>>() != ["--serve"] {
            return Err("Use the verified PhotoCraft worker protocol 2 (--serve).".into());
        }
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let runtime = executable
            .parent()
            .ok_or("Missing worker directory.")?
            .join("runtime");
        serve(runtime)
    })();
    if let Err(error) = result {
        eprintln!("BRIA worker: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_frames_reject_oversized_truncated_and_unknown_input() {
        assert!(read_frame(&mut std::io::Cursor::new(0u32.to_le_bytes())).is_err());
        assert!(read_frame(&mut std::io::Cursor::new(262145u32.to_le_bytes())).is_err());
        assert!(read_frame(&mut std::io::Cursor::new(vec![10, 0, 0, 0, b'{'])).is_err());
        let bytes = br#"{"type":"shutdown","unknown":true}"#;
        let mut frame = (bytes.len() as u32).to_le_bytes().to_vec();
        frame.extend(bytes);
        assert!(read_frame(&mut std::io::Cursor::new(frame)).is_err());
    }
}
