use std::{ops::Index, task::Poll};

use futures::StreamExt;
use sony_protocol::v1::{Packet, PacketContent, PayloadCommand1};
use sony_rs::DeviceExplorer;
use tokio_stream::wrappers::BroadcastStream;

use crate::{UiDevice, UiDeviceBattery};

pub struct DeviceStream {
    devices: Vec<(BroadcastStream<Packet>, UiDevice)>,
    device_explorer: DeviceExplorer,
}

impl Index<usize> for DeviceStream {
    type Output = UiDevice;

    fn index(&self, index: usize) -> &Self::Output {
        return &self.devices[index].1;
    }
}

impl DeviceStream {
    pub fn new(explorer: DeviceExplorer) -> Self {
        Self {
            devices: Vec::new(),
            device_explorer: explorer,
        }
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    fn handle_stream_event(device: &mut UiDevice, packet: Packet) -> anyhow::Result<()> {
        match packet.content {
            PacketContent::Command1(c) => match c {
                PayloadCommand1::AmbientSoundControlRet(n)
                | PayloadCommand1::AmbientSoundControlNotify(n) => {
                    device.anc_mode = Some(n);
                }
                PayloadCommand1::BatteryLevelReply(b) | PayloadCommand1::BatteryLevelNotify(b) => {
                    match b {
                        sony_protocol::v1::BatteryState::Single {
                            level,
                            is_charging: _,
                        } => device.battery_device = Some(UiDeviceBattery::Single(level)),
                        sony_protocol::v1::BatteryState::Case {
                            level,
                            is_charging: _,
                        } => device.battery_case = Some(level),
                        sony_protocol::v1::BatteryState::Dual {
                            level_left,
                            is_left_charging: _,
                            level_right,
                            is_right_charging: _,
                        } => {
                            device.battery_device =
                                Some(UiDeviceBattery::Dual((level_left, level_right)))
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
        Ok(())
    }
}

impl tokio_stream::Stream for DeviceStream {
    type Item = anyhow::Result<()>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut thiz = self.as_mut();
        if let Poll::Ready(r) = thiz.device_explorer.device_stream.poll_recv(cx) {
            if let Some(e) = r {
                match e {
                    sony_rs::DeviceEvent::DeviceAdded(d) => {
                        let address = d.address();

                        {
                            let d = d.clone();
                            thiz.devices.push((
                                BroadcastStream::new(d.as_ref().packets_receiver.resubscribe()),
                                UiDevice {
                                    address,
                                    device: d,
                                    anc_mode: None,
                                    battery_device: None,
                                    battery_case: None,
                                },
                            ));
                        }

                        tokio::spawn(async move {
                            d.as_ref()
                                .send(PacketContent::Command1(
                                    PayloadCommand1::AmbientSoundControlGet,
                                ))
                                .await
                                .unwrap();

                            d.as_ref()
                                .send(PacketContent::Command1(
                                    PayloadCommand1::BatteryLevelRequest(
                                        sony_protocol::v1::BatteryType::Single,
                                    ),
                                ))
                                .await
                                .unwrap();

                            d.as_ref()
                                .send(PacketContent::Command1(
                                    PayloadCommand1::BatteryLevelRequest(
                                        sony_protocol::v1::BatteryType::Dual,
                                    ),
                                ))
                                .await
                                .unwrap();

                            d.as_ref()
                                .send(PacketContent::Command1(
                                    PayloadCommand1::BatteryLevelRequest(
                                        sony_protocol::v1::BatteryType::Case,
                                    ),
                                ))
                                .await
                                .unwrap();
                        });
                    }
                    sony_rs::DeviceEvent::DeviceRemoved(_) => todo!(),
                }
                return Poll::Ready(Some(Ok(())));
            } else {
                todo!()
            }
        }

        let mut iter = thiz.devices.iter_mut();

        let mut deletable = None;
        while let Some((r, d)) = iter.next() {
            if let Poll::Ready(r) = r.poll_next_unpin(cx) {
                if let Some(r) = r {
                    Self::handle_stream_event(d, r?)?;
                    return Poll::Ready(Some(Ok(())));
                } else {
                    deletable = Some(d.address);
                    break;
                }
            }
        }

        if let Some(device) = deletable {
            thiz.devices.retain(|(_, d)| d.address != device);
            Poll::Ready(Some(Ok(())))
        } else {
            Poll::Pending
        }
    }
}
