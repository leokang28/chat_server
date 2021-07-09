use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tungstenite::{Message, Result};
use tokio_tungstenite::accept_async;
use tokio::sync::broadcast;
#[tokio::main]
async fn main() {
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:12345").await.unwrap();
    let (sender, _receiver) = broadcast::channel::<ChatMessage>(5000);
    while let Ok((stream, _addr)) = tcp_listener.accept().await {
        tokio::spawn(accept_connection(stream, sender.clone()));
    }

    println!("Hello, world!");
}

extern "C" {
    fn time(output: *mut i64) -> i64;
}

async fn accept_connection(stream: tokio::net::TcpStream, sender: broadcast::Sender<ChatMessage>) -> Result<()> {
    let ws_stream = accept_async(stream).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(10000));
    let mut receiver = sender.subscribe();
    const TIMEOUT: i64 = 15;
    let mut heartbeat: i64 = unsafe{ time(std::ptr::null_mut()) };
    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg.unwrap();
                        dbg!(&msg);
                        if msg.is_text() {
                            if let Message::Text(msg) = msg {
                                let (cmd, arg) = msg.split_once(' ').unwrap();
                                match cmd {
                                    "send" => {
                                        let chat_message = ChatMessage {
                                            msg: arg.to_string()
                                        };
                                        sender.send(chat_message).unwrap();
                                    },
                                    _ => todo!()
                                }
                            }
                            // ws_sender.send(msg).await?;
                        } else if msg.is_ping() {
                            ws_sender.send(Message::Pong(Vec::new())).await?;
                        } else if msg.is_pong() {
                            heartbeat = unsafe{ time(std::ptr::null_mut()) };
                        } else if msg.is_close() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            Ok(msg) = receiver.recv() => {
                if unsafe{ time(std::ptr::null_mut()) } - heartbeat > TIMEOUT {
                    // 断线，不发送信息
                    break;
                }

                ws_sender.send(Message::Text(msg.msg)).await?;
            }

            _ = interval.tick() => {
                ws_sender.send(Message::Ping(Vec::new())).await?;
                // ws_sender.send(Message::Text("tick".to_owned())).await?;
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct ChatMessage {
    msg: String,
    // username: String,
}

