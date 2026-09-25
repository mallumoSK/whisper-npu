use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use protocol::{ClientCommand, DaemonEvent, DaemonState, DEFAULT_SOCKET_PATH};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{error, info, warn};

pub struct IpcServer {
    socket_path: String,
    event_tx: broadcast::Sender<DaemonEvent>,
    command_tx: mpsc::Sender<ClientCommand>,
    current_state: Arc<Mutex<DaemonState>>,
}

impl IpcServer {
    pub fn new(
        socket_path: Option<String>,
        event_tx: broadcast::Sender<DaemonEvent>,
        command_tx: mpsc::Sender<ClientCommand>,
        current_state: Arc<Mutex<DaemonState>>,
    ) -> Self {
        Self {
            socket_path: socket_path.unwrap_or_else(|| DEFAULT_SOCKET_PATH.to_string()),
            event_tx,
            command_tx,
            current_state,
        }
    }

    pub async fn run(self) -> Result<()> {
        let path = Path::new(&self.socket_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create socket parent directory: {}", parent.display())
            })?;
        }

        if path.exists() {
            let _ = fs::remove_file(path);
        }

        let listener = UnixListener::bind(path)
            .with_context(|| format!("Failed to bind unix socket: {}", path.display()))?;

        info!("Daemon listening on Unix socket: {}", path.display());
        let active_clients = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    info!("Client connected to daemon socket");
                    let event_rx = self.event_tx.subscribe();
                    let command_tx = self.command_tx.clone();
                    let current_state = self.current_state.clone();
                    let active_clients = active_clients.clone();

                    tokio::spawn(async move {
                        let prev = active_clients.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        if prev == 0 {
                            info!("First client connected: starting listening and audio capture");
                            let _ = command_tx.send(ClientCommand::StartListening).await;
                        }

                        if let Err(e) = handle_client(stream, event_rx, command_tx.clone(), current_state).await {
                            warn!("Client handler finished: {:?}", e);
                        }

                        let prev = active_clients.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        if prev == 1 {
                            info!("All clients disconnected: stopping listening and entering standby (keeping model in memory)");
                            let _ = command_tx.send(ClientCommand::Stop).await;
                        }
                    });
                }
                Err(e) => {
                    error!("Socket accept error: {:?}", e);
                }
            }
        }
    }
}

async fn handle_client(
    stream: UnixStream,
    mut event_rx: broadcast::Receiver<DaemonEvent>,
    command_tx: mpsc::Sender<ClientCommand>,
    current_state: Arc<Mutex<DaemonState>>,
) -> Result<()> {
    let mut framed = Framed::new(stream, LinesCodec::new());

    // Send initial state to client
    let initial_state = *current_state.lock().await;
    let initial_event = DaemonEvent::StateChanged {
        data: initial_state,
    };
    let json = serde_json::to_string(&initial_event)?;
    framed.send(json).await?;

    loop {
        tokio::select! {
            // Receive commands from client
            line_res = framed.next() => {
                match line_res {
                    Some(Ok(line)) => {
                        match serde_json::from_str::<ClientCommand>(&line) {
                            Ok(cmd) => {
                                info!("Received command: {:?}", cmd);
                                if let Err(e) = command_tx.send(cmd).await {
                                    error!("Failed to dispatch command to audio worker: {:?}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse client command from '{}': {:?}", line, e);
                                let err_event = DaemonEvent::Error {
                                    data: format!("Invalid command: {}", e),
                                };
                                let json = serde_json::to_string(&err_event)?;
                                let _ = framed.send(json).await;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!("Framing read error: {:?}", e);
                        break;
                    }
                    None => {
                        info!("Client disconnected");
                        break;
                    }
                }
            }

            // Forward daemon events to client
            event_res = event_rx.recv() => {
                match event_res {
                    Ok(event) => {
                        let json = serde_json::to_string(&event)?;
                        if let Err(e) = framed.send(json).await {
                            warn!("Failed to send event to client: {:?}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Client lagged behind by {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
