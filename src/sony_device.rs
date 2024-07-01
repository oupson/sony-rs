use std::time::Duration;

use bluer::rfcomm::Stream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    time::timeout,
};
use tracing::{debug, trace, warn};

use crate::{
    device_session::DeviceSession,
    v1::{Packet, PacketContent},
};

struct InnerSonyDevice {
    completion_stream: Option<tokio::sync::oneshot::Sender<Packet>>,
    device_session: DeviceSession,
    device_stream: Stream,
}

pub struct SonyDevice {
    inner: tokio::sync::Mutex<InnerSonyDevice>,
}

impl SonyDevice {
    pub fn new(device_stream: Stream) -> Self {
        let inner = InnerSonyDevice {
            device_stream,
            completion_stream: None,
            device_session: DeviceSession::new(),
        };

        Self {
            inner: Mutex::new(inner),
        }
    }
    pub async fn send(&self, content: PacketContent) -> anyhow::Result<Packet> {
        let mut buffer = [0u8; 1024];

        loop {
            let mut inner = self.inner.lock().await;
            if let Some(_) = &inner.completion_stream {
                continue;
            } else {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                inner.completion_stream = Some(sender);

                let size = inner
                    .device_session
                    .encode_packet(&mut buffer, content, None)?;
                trace!("sent {:02x?}", &buffer[0..size]);

                inner.device_stream.write(&buffer[0..size]).await?;
                inner.device_stream.flush().await?;

                drop(inner);

                break Ok(receiver.await?);
            }
        }
    }

    pub async fn run(&self, sender: &tokio::sync::broadcast::Sender<Packet>) -> anyhow::Result<()> {
        let mut send_buffer = [0u8; 256];
        let mut receive_buffer = [0u8; 1024];

        loop {
            let mut inner = self.inner.lock().await;
            let size = match timeout(
                Duration::from_millis(500),
                inner.device_stream.read(&mut receive_buffer),
            )
            .await
            {
                Ok(res) => res?,
                Err(_) => continue,
            };

            let mut index = 0;
            while index < size {
                let (i, res) = inner
                    .device_session
                    .parse_packet(&receive_buffer[index..size]);

                let packet = res?;
                debug!("packet={:02x?}", packet);

                if !packet.is_ack() {
                    {
                        let size = inner.device_session.encode_packet(
                            &mut send_buffer,
                            PacketContent::Ack,
                            Some(packet.seqnum()),
                        )?;

                        inner.device_stream.write(&send_buffer[0..size]).await?;
                        inner.device_stream.flush().await?;

                        debug!("sent :{:02x?}", &send_buffer[0..size]);
                    }

                    let stream = std::mem::take(&mut inner.completion_stream);
                    if let Some(stream) = stream {
                        if let Err(packet) = stream.send(packet) {
                            warn!("failed to complete send");
                            let _ = sender.send(packet);
                        }
                    } else {
                        let _ = sender.send(packet);
                    }
                }

                index += i;
            }
        }
    }
}

mod test {

    #[test]
    fn test_parse_packets() {
        use crate::{
            device_session::DeviceSession,
            v1::{Packet, PacketContent, PayloadCommand1},
        };

        let receive = [
            62, 1, 1, 0, 0, 0, 0, 2, 60, 62, 12, 1, 0, 0, 0, 4, 1, 0, 112, 0, 130, 60,
        ];

        let mut send_buffer = [0u8; 100];

        let mut device_session = DeviceSession::new();
        device_session
            .encode_packet(
                &mut send_buffer,
                crate::v1::PacketContent::Command1(crate::v1::PayloadCommand1::InitRequest),
                None,
            )
            .unwrap();

        let (i, res) = device_session.parse_packet(&receive);
        let res = res.unwrap();
        assert_eq!(
            Packet {
                content: crate::v1::PacketContent::Ack,
                seqnum: 1
            },
            res
        );

        let (i2, res) = device_session.parse_packet(&receive[i..]);
        let res = res.unwrap();
        assert_eq!(
            Packet {
                content: PacketContent::Command1(PayloadCommand1::InitReply([0, 112, 0])),
                seqnum: 1
            },
            res
        );

        let i = i + i2;

        assert_eq!(receive.len(), i);
    }
}
