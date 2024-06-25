use std::time::Duration;

use anyhow::Context;
use bluer::rfcomm::{SocketAddr, Stream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};
use v1::{AncPacket, Payload};

mod v1;

#[derive(Debug)]
#[repr(u8)]
pub enum Datatype {
    Ack = 0x1,
    Command1 = 0x0c,
    Command2 = 0x0e,
    Unknown = 0xff,
}

impl From<u8> for Datatype {
    fn from(value: u8) -> Self {
        match (value) {
            0x1 => Self::Ack,
            0x0c => Self::Command1,
            0x0e => Self::Command2,
            _ => Self::Unknown,
        }
    }
}

pub struct Packet<P>
where
    P: Payload,
{
    data_type: Datatype,
    seqnum: u8,
    payload: P,
}

impl<P> Packet<P>
where
    P: Payload,
{
    pub fn write_into(self, buf: &mut [u8]) -> anyhow::Result<usize> {
        buf[0] = 0x3e;
        buf[1] = self.data_type as u8;
        buf[2] = self.seqnum;
        let size = self.payload.write_into(&mut buf[7..])?;
        buf[3..7].copy_from_slice(&size.to_be_bytes());

        let end = 7 + size as usize;

        let checksum = buf[1..end]
            .iter()
            .fold(0, |acc: u8, x: &u8| acc.wrapping_add(*x));

        buf[end] = checksum;

        buf[end + 1] = 0x3d;

        Ok(end + 2)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = bluer::Session::new().await?;

    let adapter = session.default_adapter().await?;

    let addrs = adapter.device_addresses().await?;
    for addr in addrs {
        let device = adapter.device(addr)?;
        if device.is_paired().await? && device.is_connected().await? {
            let name = device.name().await?;

            println!("{:?}", name);

            if name == Some("LE_WF-1000XM3".to_owned()) {
                println!("{:?}", device.address());
                let props = device.uuids().await?;

                println!("{:?}", props);

                break;
                let target_sa = SocketAddr::new(device.address(), 9);
                let mut channel = Stream::connect(target_sa).await?;

                tokio::time::sleep(Duration::from_millis(500)).await;

                let packet = Packet::<&[u8]> {
                    data_type: Datatype::Command1,
                    seqnum: 0,
                    payload: &[0, 0],
                };

                let mut buffer = vec![0; 1024];

                let size = packet.write_into(&mut buffer)?;

                for i in 0..3 {
                    let size = channel
                        .write(&buffer[0..size])
                        .await
                        .context("failed to send message")?;

                    match timeout(Duration::from_secs(1), channel.read(&mut buffer)).await {
                        Ok(res) => {
                            println!("{:?}", &buffer[0..res?]);
                            break;
                        }
                        Err(_) => {}
                    };
                }

                let payload = AncPacket {
                    anc_mode: v1::AncMode::Off,
                    focus_on_voice: false,
                    ambiant_level: 0,
                };

                let packet = Packet {
                    data_type: Datatype::Command1,
                    seqnum: 2,
                    payload,
                };

                let mut buf = [0u8; 17];

                let size = packet.write_into(&mut buf)?;

                for _ in 0..3 {
                    println!("sending !");
                    let size = channel
                        .write(&buf[0..size])
                        .await
                        .context("failed to send message")?;

                    println!("{}", size);

                    let mut buffer = vec![0; 1024];

                    match timeout(Duration::from_secs(1), channel.read(&mut buffer)).await {
                        Ok(res) => {
                            println!("{:?}", &buffer[0..res?]);
                            break;
                        }
                        Err(_) => {}
                    };
                }
                drop(channel);
            }
        }
    }

    Ok(())
}
