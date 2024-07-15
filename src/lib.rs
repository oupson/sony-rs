use bluer::{
    agent::Agent,
    rfcomm::{Profile, Role, Stream},
    AdapterEvent, Address, ErrorKind,
};
use futures::StreamExt;
pub use sony_device::SonyDevice;
use sony_protocol::v1::{PacketContent, PayloadCommand1};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};
use tracing::{error, warn};

mod sony_device;

pub struct Device {
    address: Address,
    sony_device: SonyDevice,
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Device")
            .field("address", &self.address)
            .finish()
    }
}

impl Device {
    pub fn address(&self) -> Address {
        self.address
    }
}

impl AsRef<SonyDevice> for Device {
    fn as_ref(&self) -> &SonyDevice {
        return &self.sony_device;
    }
}

#[derive(Debug)]
pub enum DeviceEvent {
    DeviceAdded(Device),
    DeviceRemoved(Address),
}

pub struct DeviceExplorer {
    pub device_stream: Receiver<DeviceEvent>,
    join_handle: JoinHandle<()>,
}

impl DeviceExplorer {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel(1);

        let handle = tokio::spawn(async move {
            if let Err(e) = run_loop(sender).await {
                error!("something failed : {}", e);
            }
        });

        Self {
            device_stream: receiver,
            join_handle: handle,
        }
    }

    pub fn device_stream(&mut self) -> &mut Receiver<DeviceEvent> {
        &mut self.device_stream
    }
}

async fn run_loop(sender: Sender<DeviceEvent>) -> anyhow::Result<()> {
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
                        while let Err(e) = device.connect_profile(&profile_uuid).await {
                            if e.kind != ErrorKind::InProgress {
                                warn!("failed to connect to profile : {}", e);
                                break;
                            }
                        }

                    });
                }
            }

            request = hndl.next() => {
                if let Some(r) = request {
                    let addr = r.device();
                    let channel = r.accept().unwrap();
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        match start_communication(channel).await {
                            Ok(device) => {
                                 _ = sender.send(DeviceEvent::DeviceAdded(Device { address: addr, sony_device: device })).await;
                            }
                            Err(e) => error!("failed to connect to device : {}", e),
                        }
                    });
                }
            }
        }
    }
}

async fn start_communication(channel: Stream) -> anyhow::Result<SonyDevice> {
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

    tracing::debug!("foo");

    Ok(device)
}
