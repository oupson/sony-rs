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

use v1::{AncPayload, Datatype, Packet};

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

        let payload = AncPayload {
            anc_mode: v1::AncMode::On,
            focus_on_voice: false,
            ambiant_level: 1,
        };

        let packet = Packet {
            data_type: Datatype::Command1,
            seqnum: 0,
            payload,
        };

        let size = packet.write_into(&mut buffer)?;
        println!("sending {:x?}", &buffer[0..size]);

        channel
            .write(&buffer[0..size])
            .await
            .context("failed to send message")?;

        match timeout(Duration::from_secs(1), channel.read(&mut buffer)).await {
            Ok(res) => {
                println!("read {:x?}", &buffer[0..res?]);
            }
            Err(_) => {}
        };

        channel.shutdown().await?;
    }

    Ok(())
}
