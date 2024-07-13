use std::time::Duration;

use anyhow::Context;
use bluer::rfcomm::Stream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    time::{self, Instant},
};
use tracing::trace;

use crate::v1::{Packet, PacketContent};

struct InnerSonyDevice {
    completion_stream: Option<tokio::sync::oneshot::Sender<()>>,
    device_session: sony_protocol::Device,
}

pub struct SonyDevice {
    inner: tokio::sync::Mutex<InnerSonyDevice>,
}

impl SonyDevice {
    pub fn new() -> Self {
        let inner = InnerSonyDevice {
            completion_stream: None,
            device_session: Default::default(),
        };

        Self {
            inner: Mutex::new(inner),
        }
    }

    pub async fn send(&self, content: PacketContent) -> anyhow::Result<()> {
        loop {
            let mut inner = self.inner.lock().await;
            if let Some(_) = &inner.completion_stream {
                continue;
            } else {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                inner.completion_stream = Some(sender);
                inner.device_session.send_packet(content)?;
                drop(inner);
                receiver.await?;
                break Ok(());
            }
        }
    }

    pub async fn run(
        &self,
        mut device_stream: Stream,
        sender: &tokio::sync::broadcast::Sender<Packet>,
    ) -> anyhow::Result<()> {
        let mut receive_buffer = [0u8; 1024];

        let next_poll = time::sleep(Duration::from_secs(0));
        tokio::pin!(next_poll);

        loop {
            tokio::select! {
                res = device_stream.read(&mut receive_buffer) => {
                    let num_read = res.context("receive failed")?;
                    let mut inner = self.inner.lock().await;
                    inner.device_session.received_packet(&receive_buffer[..num_read])?;
                    let state = inner.device_session.poll()?;
                    trace!("recv: state = {:?}", state);

                    match state {
                        sony_protocol::State::WaitingPacket(next) => {
                            if let Some(next) = next {
                                next_poll.as_mut().reset(next.into());
                            } else {
                                next_poll
                                    .as_mut()
                                    .reset(Instant::now() + Duration::from_millis(500));
                            }
                        }
                        sony_protocol::State::ReceivedPacket(p) => match p.content {
                            PacketContent::Ack => {
                                if let Some(c) = inner.completion_stream.take(){
                                    _ = c.send(());
                                }
                            }
                            _ => {
                                tracing::trace!("sending {:?}", p);
                                sender.send(p)?;
                            }
                        },
                        sony_protocol::State::SendPacket(p) => {
                            device_stream.write(p).await?;
                        }
                    };
                    drop(inner)
                }
                _ = &mut next_poll => {
                    let mut inner = self.inner.lock().await;
                    let state =  inner.device_session.poll()?;
                    match state {
                        sony_protocol::State::WaitingPacket(next) => {
                            if let Some(next) = next {
                                next_poll.as_mut().reset(next.into());
                            } else {
                                next_poll
                                    .as_mut()
                                    .reset(Instant::now() + Duration::from_millis(50));
                            }
                        }
                        sony_protocol::State::ReceivedPacket(p) => match p.content {
                            PacketContent::Ack => {
                                if let Some(c) = inner.completion_stream .take(){
                                    _ = c.send(());
                                }
                            }
                            _ => {
                                tracing::trace!("sending 1 {:?}", p);
                                sender.send(p)?;
                            }
                        },
                        sony_protocol::State::SendPacket(p) => {
                            device_stream.write(p).await.context("failed to send")?;
                        }
                    };
                    drop(inner)
                },

            };
        }
    }
}
