use std::error::Error;
use std::str::from_utf8;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast};
use zbus::{Connection, connection, interface};


struct ClipboardSyncDaemon {
    lastclip: String,
    tx: broadcast::Sender<String>,
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

async fn push_to_gnome_clipboard(dbus_conn: &Connection, text: String) -> Result<(), Box<dyn Error>> {
    dbus_conn.call_method(
        Some("org.gnome.Shell"), // Target container
        "/com/clipsync/Extension", // The path we defined in extension.js
        Some("com.clipsync.Extension"), // The interface name
        "SetClipboard", // The method name
        &(text,),
    ).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (tx, _) = broadcast::channel::<String>(16);

    let daemon_state = ClipboardSyncDaemon{lastclip: "".to_string(), tx: tx.clone()};

    println!("Initializing D-Bus Session Connection ...");

    let conn = connection::Builder::session()?
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
            if let Ok((socket, addr)) = listener.accept().await {
                println!("Device connected from IP: {}", addr);

                let (mut reader, mut writer) = io::split(socket);

                let mut rx_client = tx.subscribe();

                tokio::spawn(async move {
                    loop {
                        while let Ok(clipboard_text) = rx_client.recv().await {
                            println!("Sending clipboard to network stream: {}", clipboard_text);

                            let payload = format!("{}\n", clipboard_text);
                            if let Err(e) = writer.write_all(payload.as_bytes()).await {
                                eprintln!("Failed to write data to network socket: {}", e);
                                break;
                            }
                        }
                    }
                });
                let conn_clone = conn.clone();
                tokio::spawn(async move {
                    let mut buffer = [0;1024];
                    loop {
                        match reader.read(&mut buffer).await {
                            Ok(0) => {
                                println!("Device disconnected: {}", addr);
                            }
                            Ok(n) => {
                                if let Ok(incoming_text) = from_utf8(&buffer[..n]) {
                                    let clean_text = incoming_text.trim().to_string();
                                    if !clean_text.is_empty() {
                                        println!("Network received remote clip: '{}'", clean_text);

                                        if let Ok(ref interface_ref) = conn_clone.object_server()
                                        .interface::<_, ClipboardSyncDaemon>("/com/clipsync/Daemon").await {
                                            let mut daemon = interface_ref.get_mut().await;
                                            daemon.lastclip = clean_text.clone();
                                        }

                                        if let Err(e) = push_to_gnome_clipboard(&conn_clone, clean_text).await {
                                            eprintln!("Failed to push incoming network text to GNOME: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Socket read error: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
        }
    });
    
    std::future::pending::<()>().await;

    Ok(())
}
