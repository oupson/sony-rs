use std::time::Duration;

use anyhow::Context;
use bluer::rfcomm::Stream;
use futures::Future;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{
        broadcast::{self, Receiver as BroadcastReceiver, Sender as BroadcastSender},
        mpsc::{self, Receiver as MpscReceiver, Sender as MspcSender},
        oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender},
    },
    time::{self, Instant},
};
use tracing::trace;

use crate::v1::{Packet, PacketContent};

pub struct SonyDevice {
    pub packets_queries: MspcSender<(PacketContent, OneshotSender<()>)>,
    pub packets_receiver: BroadcastReceiver<Packet>,
}

impl SonyDevice {
    pub fn new(device_stream: Stream) -> (Self, impl Future<Output = anyhow::Result<()>>) {
        let (sender, receiver) = mpsc::channel(1);

        let (broadcast_sender, broadcast_receiver) = broadcast::channel(1);

        let thiz = Self {
            packets_queries: sender,
            packets_receiver: broadcast_receiver,
        };

        let run = Self::run(device_stream, receiver, broadcast_sender);
        (thiz, run)
    }

    pub async fn send(&self, content: PacketContent) -> anyhow::Result<OneshotReceiver<()>> {
        let (sender, receiver) = oneshot::channel();
        self.packets_queries.send((content, sender)).await?;
        Ok(receiver)
    }

    pub async fn run(
        mut device_stream: Stream,
        mut next_packets: MpscReceiver<(PacketContent, OneshotSender<()>)>,
        sender: BroadcastSender<Packet>,
    ) -> anyhow::Result<()> {
        let mut device_session = sony_protocol::Device::default();
        let mut receive_buffer = [0u8; 1024];

        let next_poll = time::sleep(Duration::from_secs(0));
        tokio::pin!(next_poll);

        let mut next_packet = None;

        loop {
            let read = tokio::select! {
                res = device_stream.read(&mut receive_buffer) => {
                    let num_read = res.context("receive failed")?;
                    Some(num_read)
                }
                next = next_packets.recv(), if next_packet.is_none() => {
                    if let Some((p, c)) = next {
                        device_session.send_packet(p)?;
                        next_packet = Some(c);
                    }
                    None
                },
                _ = &mut next_poll => None,
            };

            if let Some(num_read) = read {
                device_session.received_packet(&receive_buffer[..num_read])?;
            }

            let wait = loop {
                let state = device_session.poll()?;
                trace!("run_loop: state = {:?}", state);

                match state {
                    sony_protocol::State::WaitingPacket(next) => {
                        break next;
                    }
                    sony_protocol::State::ReceivedPacket(p) => match p.content {
                        PacketContent::Ack => {
                            if let Some(c) = next_packet.take() {
                                _ = c.send(());
                            }
                        }
                        _ => {
                            tracing::trace!("run_loop: sending to broadcast packet={:?}", p);
                            sender.send(p)?;
                        }
                    },
                    sony_protocol::State::SendPacket(p) => {
                        device_stream.write(p).await?;
                    }
                };
            };
            if let Some(wait) = wait {
                next_poll.as_mut().reset(wait.into());
            } else {
                next_poll
                    .as_mut()
                    .reset(Instant::now() + Duration::from_secs(10));
            }
        }
    }
}
