use bluer::Address;
use sony_protocol::v1::{AncPayload, Packet, PacketContent, PayloadCommand1};
use sony_rs::{Device, DeviceEvent};
use tokio_stream::{wrappers::BroadcastStream, StreamExt, StreamMap};
use tracing::debug;

struct UiDevice {
    address: Address,
    device: Device,
    anc_mode: Option<AncPayload>,
}

struct App {
    devices: Vec<UiDevice>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
        }
    }
}

impl App {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut event_streams = StreamMap::new();

        let sony_explorer = sony_rs::DeviceExplorer::start();

        let mut device_stream = sony_explorer.device_stream;

        loop {
            let _ = tokio::select! {
                Some(event) = device_stream.recv() => {
                    self.handle_explorer_event(event, &mut event_streams).await?;
                }
                Some((address, event)) = event_streams.next() => {
                    if let Ok(packet) = event {
                        self.handle_device_event(address, packet)?;
                    }
                }
            };

            for d in self.devices.iter() {
                println!("{} : {:?}", d.address, d.anc_mode);
            }
        }
    }

    async fn handle_explorer_event(
        &mut self,
        event: DeviceEvent,
        event_streams: &mut StreamMap<Address, BroadcastStream<Packet>>,
    ) -> anyhow::Result<()> {
        debug!("{:?}", event);
        match event {
            sony_rs::DeviceEvent::DeviceAdded(d) => {
                d.as_ref()
                    .send(PacketContent::Command1(
                        PayloadCommand1::AmbientSoundControlGet,
                    ))
                    .await?;

                d.as_ref()
                    .send(PacketContent::Command1(
                        PayloadCommand1::BatteryLevelRequest(sony_protocol::v1::BatteryType::Dual),
                    ))
                    .await?;

                d.as_ref()
                    .send(PacketContent::Command1(
                        PayloadCommand1::BatteryLevelRequest(sony_protocol::v1::BatteryType::Case),
                    ))
                    .await?;

                let address = d.address();

                event_streams.insert(
                    address,
                    tokio_stream::wrappers::BroadcastStream::new(
                        d.as_ref().packets_receiver.resubscribe(),
                    ),
                );

                self.devices.push(UiDevice {
                    address,
                    device: d,
                    anc_mode: None,
                });
            }
            sony_rs::DeviceEvent::DeviceRemoved(_) => todo!(),
        }
        Ok(())
    }

    fn handle_device_event(&mut self, address: Address, event: Packet) -> anyhow::Result<()> {
        match event.content {
            PacketContent::Command1(c) => match c {
                PayloadCommand1::AmbientSoundControlRet(n)
                | PayloadCommand1::AmbientSoundControlNotify(n) => {
                    self.devices
                        .iter_mut()
                        .find(|c| c.address == address)
                        .unwrap()
                        .anc_mode = Some(n);
                }
                _ => (),
            },
            _ => (),
        }

        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut app = App::default();
    app.run().await?;
    Ok(())
}
