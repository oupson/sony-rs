use std::{sync::Arc, time::Duration};

use bluer::{
    agent::Agent,
    rfcomm::{Profile, Role},
};
use futures::StreamExt;
use sony_device::SonyDevice;
use tracing::{info, trace};

mod device_session;
mod sony_device;
mod v1;

use v1::{AncMode, AncPayload, PacketContent, PayloadCommand1};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

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
        let channel = r.accept()?;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let device = Arc::new(SonyDevice::new(channel));

        let mut receiver = {
            let device = device.clone();
            let (sender, receiver) = tokio::sync::broadcast::channel(1);

            tokio::spawn(async move {
                device.run(sender).await.unwrap();
            });

            receiver
        };

        let res = device
            .send(PacketContent::Command1(PayloadCommand1::InitRequest))
            .await?;

        trace!("InitRequest = {:?}", res);

        let res = device
            .send(PacketContent::Command1(
                PayloadCommand1::AmbientSoundControlGet,
            ))
            .await?;

        trace!("AmbientSoundControlGet = {:?}", res);

        let anc_mode =
            if let PacketContent::Command1(PayloadCommand1::AmbientSoundControlRet(res)) =
                res.content
            {
                res
            } else {
                todo!()
            };

        let res = device
            .send(PacketContent::Command1(
                PayloadCommand1::AmbientSoundControlSet(AncPayload {
                    anc_mode: if anc_mode.anc_mode == AncMode::Off {
                        AncMode::On
                    } else {
                        AncMode::Off
                    },
                    focus_on_voice: false,
                    ambiant_level: 0,
                }),
            ))
            .await?;

        trace!("AmbientSoundControlSet = {:?}", res);

        while let Ok(event) = receiver.recv().await {
            info!("new event : {:?}", event);
        }
    }
    Ok(())
}
