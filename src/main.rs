use std::error::Error;
use tokio::io::{AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use zbus::{connection, interface};


struct ClipboardSyncDaemon {
    lastclip: String,
    tx: mpsc::UnboundedSender<String>,
}

#[interface(name = "com.clipsync.Daemon")]
impl ClipboardSyncDaemon {
    async fn update_clipboard(&mut self, text: String) {
        if !text.is_empty() && text != self.lastclip {
            println!("D-Bus Daemon received new Text: {}", text);
            self.lastclip = text.clone();

            if let Err(e) = self.tx.send(text) {
                eprintln!("Could not send text: {}", e)
            }
        }
    }

    // A Property: Clients can query state directly
    #[zbus(property)]
    async fn version(&self) -> &str {
        "1.0.0"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let daemon_state = ClipboardSyncDaemon{lastclip: "".to_string(), tx};

    println!("Initializing D-Bus Session Connection ...");

    let _conn = connection::Builder::session()?
        .name("com.clipsync.Daemon")?
        .serve_at("/com/clipsync/Daemon", daemon_state)?
        .build()
        .await?;

    println!("Daemon is successfully listening on D-Bus!");



    // Tcp Listener
    let listener = TcpListener::bind("0.0.0.0:6789").await?;
    println!("Tokio P2P Server listening on port 6789 ...");

    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, addr)) = listener.accept().await {
                println!("Device connected from IP: {}", addr);

                while let Some(clipboard_text) = rx.recv().await {
                    println!("Sending clipboard to network stream: {}", clipboard_text);

                    let payload = format!("{}\n", clipboard_text);
                    if let Err(e) = socket.write_all(payload.as_bytes()).await {
                        eprintln!("Failed to write data to network socket: {}", e);
                        break;
                    }
                }
                println!("Connection closed with: {}", addr);
            }
        }
    });
    
    std::future::pending::<()>().await;

    Ok(())
}
