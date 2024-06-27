use std::time::Duration;

use anyhow::Context;
use bluer::{
    agent::Agent,
    rfcomm::{Profile, Role},
};
use futures::StreamExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

mod v1;

use v1::{AllPayload, AncPayload, Datatype, GetAnc, Packet, PayloadCommand1};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let session = bluer::Session::new().await?;

    let agent = Agent::default();
    let _agent_hndl = session.register_agent(agent).await?;

    let profile = Profile {
        uuid: uuid::uuid!("96CC203E-5068-46ad-B32D-E316F5E069BA"),
        role: Some(Role::Client),
        auto_connect: Some(true),
        require_authorization: Some(false),
        require_authentication: Some(false),
        ..Default::default()
    };

    let mut hndl = session.register_profile(profile).await?;

    let request = hndl.next().await;

    if let Some(r) = request {
        let mut channel = r.accept()?;

        let mut buffer = [0u8; 1024];

        let mut ack_to_send = 0;

        let mut seqnum = 0;

        {
            let packet = Packet::Command1(seqnum, PayloadCommand1::InitRequest);

            let size = packet.write_into(&mut buffer)?;
            println!("sending {:02x?}", &buffer[0..size]);

            channel
                .write(&buffer[0..size])
                .await
                .context("failed to send message")?;

            match timeout(Duration::from_secs(1), channel.read(&mut buffer)).await {
                Ok(r) => {
                    let size = r?;
                    println!("read : {:02x?}", &buffer[0..size]);
                    let packets = parse_packets(&buffer[0..size])?;
                    println!("{:02x?}", packets);

                    for packet in packets {
                        if !packet.is_ack() {
                            ack_to_send += 1;
                        }

                        seqnum = packet.seqnum();
                    }
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        while ack_to_send > 0 {
            let packet = Packet::Ack(seqnum);

            let size = packet.write_into(&mut buffer)?;
            println!("sending {:02x?}", &buffer[0..size]);

            channel
                .write(&buffer[0..size])
                .await
                .context("failed to send message")?;

            ack_to_send -= 1;
        }

        {
            let packet = Packet::Command1(seqnum, PayloadCommand1::AmbientSoundControlGet);

            let size = packet.write_into(&mut buffer)?;
            println!("sending {:02x?}", &buffer[0..size]);

            channel
                .write(&buffer[0..size])
                .await
                .context("failed to send message")?;

            let size = timeout(Duration::from_secs(1), channel.read(&mut buffer)).await??;
            println!("read : {:02x?}", &buffer[0..size]);

            let packets = parse_packets(&buffer[0..size])?;
            println!("{:02x?}", packets);
        }

        let payload = AncPayload {
            anc_mode: v1::AncMode::On,
            focus_on_voice: false,
            ambiant_level: 1,
        };

        let packet = Packet::Command1(seqnum, PayloadCommand1::AmbientSoundControlSet(payload));

        let size = packet.write_into(&mut buffer)?;
        println!("sending {:02x?}", &buffer[0..size]);

        channel
            .write(&buffer[0..size])
            .await
            .context("failed to send message")?;

        let size = timeout(Duration::from_secs(1), channel.read(&mut buffer)).await??;
        println!("read : {:02x?}", &buffer[0..size]);

        let packets = parse_packets(&buffer[0..size])?;
        println!("{:02x?}", packets);

        channel.shutdown().await?;
    }

    Ok(())
}

fn parse_packets(buf: &[u8]) -> anyhow::Result<Vec<Packet>> {
    let mut res = Vec::new();
    for msg in buf.split_inclusive(|c| *c == 60) {
        let packet = Packet::try_from(msg)?;
        res.push(packet)
    }

    Ok(res)
}
