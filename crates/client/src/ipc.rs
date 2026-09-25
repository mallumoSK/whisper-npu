use futures::{SinkExt, StreamExt};
use protocol::{ClientCommand, DaemonEvent, DEFAULT_SOCKET_PATH};
use std::sync::mpsc::Receiver;
use tokio::net::UnixStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{debug, error, info, warn};

pub struct ClientIpcHandle {
    pub cmd_tx: UnboundedSender<ClientCommand>,
    pub event_rx: Receiver<DaemonEvent>,
}

pub fn start_ipc_client(socket_path: Option<String>) -> ClientIpcHandle {
    let path = socket_path.unwrap_or_else(|| DEFAULT_SOCKET_PATH.to_string());
    let (gui_cmd_tx, mut worker_cmd_rx) = unbounded_channel::<ClientCommand>();
    let (gui_event_tx, gui_event_rx) = std::sync::mpsc::channel::<DaemonEvent>();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create tokio runtime for IPC client: {:?}", e);
                return;
            }
        };

        rt.block_on(async move {
            loop {
                info!("Attempting connection to daemon at {}", path);
                match UnixStream::connect(&path).await {
                    Ok(stream) => {
                        info!("Connected to whisper-daemon");
                        let mut framed: Framed<UnixStream, LinesCodec> =
                            Framed::new(stream, LinesCodec::new());

                        loop {
                            tokio::select! {
                                cmd_opt = worker_cmd_rx.recv() => {
                                    match cmd_opt {
                                        Some(cmd) => {
                                            if let Ok(json) = serde_json::to_string(&cmd) {
                                                if let Err(e) = framed.send(json).await {
                                                    warn!("Failed to send command to daemon: {:?}", e);
                                                    break;
                                                }
                                            }
                                        }
                                        None => {
                                            // GUI exited and dropped channel
                                            return;
                                        }
                                    }
                                }
                                res = framed.next() => {
                                    match res {
                                        Some(Ok(msg)) => {
                                            match serde_json::from_str::<DaemonEvent>(&msg) {
                                                Ok(event) => {
                                                    let _ = gui_event_tx.send(event);
                                                }
                                                Err(e) => {
                                                    warn!("Failed to parse daemon event: {:?} (raw: {})", e, msg);
                                                }
                                            }
                                        }
                                        Some(Err(e)) => {
                                            warn!("Daemon connection error: {:?}", e);
                                            break;
                                        }
                                        None => {
                                            info!("Daemon connection closed");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Cannot connect to daemon ({}): {:?}. Retrying in 1s...", path, e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    });

    ClientIpcHandle {
        cmd_tx: gui_cmd_tx,
        event_rx: gui_event_rx,
    }
}
