use tracing::trace;

use crate::v1::{Packet, PacketContent};

pub struct DeviceSession {
    seqnum: u8,
    pending_ack: u8,
}

impl DeviceSession {
    pub fn new() -> Self {
        Self {
            seqnum: 0,
            pending_ack: 0,
        }
    }

    pub fn parse_packet(&mut self, buffer: &[u8]) -> (usize, anyhow::Result<Packet>) {
        if let Some(index) = buffer.iter().position(|p| *p == 60) {
            let packet = Packet::try_from(&buffer[0..index]);
            if let Ok(packet) = &packet {
                if packet.is_ack() && self.pending_ack > 0 {
                    self.pending_ack -= 1;
                    self.seqnum = packet.seqnum();
                }
            }
            (index + 1, packet)
        } else {
            (
                buffer.len(),
                Err(anyhow::format_err!("no packet in buffer")),
            )
        }
    }

    pub fn encode_packet(
        &mut self,
        buffer: &mut [u8],
        command: PacketContent,
        seqnum: Option<u8>,
    ) -> anyhow::Result<usize> {
        trace!(
            "encodepacket pending_ack={}, command={:?}, seqnum = {}",
            self.pending_ack,
            command,
            self.seqnum
        );

        let seqnum = if command != PacketContent::Ack {
            if self.pending_ack > 0 {
                return Err(anyhow::format_err!("already awaiting response"));
            } else {
                self.pending_ack += 1;
            }

            seqnum.unwrap_or(self.seqnum)
        } else {
            1u8.wrapping_sub(seqnum.unwrap_or(self.seqnum))
        };

        let packet = Packet::new(seqnum, command);

        return packet.write_into(buffer);
    }
}
