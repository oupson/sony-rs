use std::time::Duration;

use anyhow::Context;
use bluer::{
    agent::Agent,
    rfcomm::{Profile, Role, Stream},
    AdapterEvent,
};
use futures::StreamExt;
use sony_device::SonyDevice;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, info};

mod sony_device;

use sony_protocol::v1;

use v1::{AncMode, AncPayload, PacketContent, PayloadCommand1};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;

    let agent = Agent::default();
    let _agent_hndl = session.register_agent(agent).await?;

    let profile_uuid = uuid::uuid!("96CC203E-5068-46ad-B32D-E316F5E069BA");

    let profile = Profile {
        uuid: profile_uuid,
        role: Some(Role::Client),
        auto_connect: Some(true),
        require_authorization: Some(false),
        require_authentication: Some(false),
        ..Default::default()
    };

    let mut hndl = session.register_profile(profile).await?;

    let events = adapter.events().await?;
    tokio::pin!(events);

    loop {
        tokio::select! {
            event = events.next() => {
                if let Some(AdapterEvent::DeviceAdded(dev)) = event {
                        let device = adapter.device(dev)?;
                        tokio::spawn(async move {
                            let _ = device.connect().await;
                            let _ = device.connect_profile(&profile_uuid).await;
                        });
                }
            }

            request = hndl.next() => {
                if let Some(r) = request {
                    let channel = r.accept()?;
                    tokio::spawn(async move {
                        if let Err(e) = device_loop(channel).await {
                            error!("while connected to device : {}", e);
                        }
                    });
                }
            }
        }
    }
}

async fn device_loop(channel: Stream) -> anyhow::Result<()> {
    tokio::time::sleep(Duration::from_secs(1)).await;

    let (mut device, run_loop) = SonyDevice::new(channel);

    tokio::spawn(async move {
        if let Err(e) = run_loop.await {
            error!("on device loop : {}", e);
        }
    });

    device
        .send(PacketContent::Command1(PayloadCommand1::InitRequest))
        .await?;

    _ = device.packets_receiver.recv().await?;

    device
        .send(PacketContent::Command1(
            PayloadCommand1::AmbientSoundControlGet,
        ))
        .await?;

    let anc_mode = loop {
        let res = device.packets_receiver.recv().await?;

        if let PacketContent::Command1(PayloadCommand1::AmbientSoundControlRet(res)) = res.content {
            break res;
        }
    };

    info!("AmbientSoundControlGet = {:?}", anc_mode);

    device
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

    info!(
        "recv : {:?}",
        device.packets_receiver.recv().await.context("failed")?
    );

    device
        .send(PacketContent::Command1(
            PayloadCommand1::BatteryLevelRequest(v1::BatteryType::Single),
        ))
        .await?;

    info!(
        "recv : {:?}",
        device.packets_receiver.recv().await.context("failed")?
    );

    device
        .send(PacketContent::Command1(
            PayloadCommand1::BatteryLevelRequest(v1::BatteryType::Dual),
        ))
        .await?;

    info!(
        "recv : {:?}",
        device.packets_receiver.recv().await.context("failed2")?
    );

    device
        .send(PacketContent::Command1(
            PayloadCommand1::BatteryLevelRequest(v1::BatteryType::Case),
        ))
        .await?;

    info!(
        "recv : {:?}",
        device.packets_receiver.recv().await.context("failed3")?
    );

    loop {
        match device.packets_receiver.recv().await {
            Ok(ev) => {
                info!("new event : {:?}", ev)
            }
            Err(RecvError::Closed) => {
                break;
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }
    Ok(())
}
