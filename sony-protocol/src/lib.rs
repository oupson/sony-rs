use std::{
    ops::{Not, Range},
    time::{Duration, Instant},
};

use tracing::trace;
use v1::{Packet, PacketContent};

pub mod v1;

#[derive(Debug)]
pub enum State<'a> {
    WaitingPacket(Option<Instant>),
    ReceivedPacket(crate::v1::Packet),
    SendPacket(&'a [u8]),
}

const MESSAGE_HEADER: u8 = 0x3e;
const MESSAGE_TRAILER: u8 = 0x3c;
const MESSAGE_ESCAPE: u8 = 0x3d;
const MESSAGE_ESCAPE_MASK: u8 = 0b11101111;

#[derive(Debug)]
pub struct Device {
    pending_packet: Option<Packet>,
    read_buf: [u8; 1024],
    write_buf: [u8; 1024],
    reading: Option<(usize, usize)>,
    sending: Option<(Range<usize>, Option<Instant>, Duration)>,
    seqnum: u8,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            pending_packet: None,
            read_buf: [0u8; 1024],
            write_buf: [0u8; 1024],
            reading: None,
            sending: None,
            seqnum: 0,
        }
    }
}

const RETRY_DURATION: Duration = Duration::from_secs(1);

impl Device {
    pub fn received_packet(&mut self, content: &[u8]) -> anyhow::Result<usize> {
        trace!("received {:02x?}", content);
        let (start, mut index) = self.reading.unwrap_or((0, 0));

        let mut content_index = 0;
        while content_index < content.len() && index < self.read_buf.len() - 1 {
            if content[content_index] == MESSAGE_ESCAPE {
                self.read_buf[index] = content[content_index + 1] | MESSAGE_ESCAPE_MASK.not();
                content_index += 2;
            } else {
                self.read_buf[index] = content[content_index];
                content_index += 1;
            }

            index += 1;
        }

        self.reading = Some((start, index));

        return Ok(content_index);
    }

    pub fn poll<'a>(&'a mut self) -> anyhow::Result<State<'a>> {
        if let Some(packet) = self.pending_packet.take() {
            Ok(State::ReceivedPacket(packet))
        } else if let Some((start, end)) = self.reading {
            let pos = start
                + self.read_buf[start..end]
                    .iter()
                    .position(|c| *c == MESSAGE_TRAILER)
                    .unwrap()
                + 1;

            let packet = v1::Packet::try_from(&self.read_buf[start..pos])?;

            self.reading = if pos == end { None } else { Some((pos, end)) };

            // TODO IGNORE SEQNUM ALREADY
            if !packet.is_ack() {
                let seqnum = packet.seqnum();

                self.pending_packet = Some(packet);

                let size = self.encode_packet(PacketContent::Ack, Some(seqnum))?;

                Ok(State::SendPacket(&self.write_buf[size]))
            } else {
                if self.sending.is_some() {
                    self.seqnum = packet.seqnum();
                    self.sending = None;
                }

                Ok(State::ReceivedPacket(packet))
            }
        } else if let Some((r, i, d)) = self.sending.take() {
            if let Some(i) = i {
                if i.elapsed() > RETRY_DURATION {
                    self.sending = Some((r.clone(), Some(Instant::now() + d), d));
                    Ok(State::SendPacket(&self.write_buf[r.clone()]))
                } else {
                    self.sending = Some((r, Some(i), d));
                    Ok(State::WaitingPacket(Some(i.clone())))
                }
            } else {
                self.sending = Some((r.clone(), Some(Instant::now() + d), d));
                Ok(State::SendPacket(&self.write_buf[r.clone()]))
            }
        } else {
            Ok(State::WaitingPacket(None))
        }
    }

    fn encode_packet(
        &mut self,
        command: PacketContent,
        seqnum: Option<u8>,
    ) -> anyhow::Result<Range<usize>> {
        let seqnum = if command != PacketContent::Ack {
            if self.sending.is_some() {
                return Err(anyhow::format_err!("already awaiting response"));
            }

            seqnum.unwrap_or(self.seqnum)
        } else {
            1u8.wrapping_sub(seqnum.unwrap_or(self.seqnum))
        };

        let packet = Packet::new(seqnum, command);

        let start = self.sending.as_ref().map(|(c, _, _)| c.end).unwrap_or(0);

        return Ok(start..start + packet.write_into(&mut self.write_buf[start..])?);
    }

    pub fn send_packet(&mut self, content: PacketContent) -> anyhow::Result<()> {
        trace!("send_packet : {:?}", content);
        if self.sending.is_some() {
            return Err(anyhow::format_err!("already awaiting response"));
        } else {
            let seq = self.encode_packet(content, None)?;
            self.sending = Some((seq, None, RETRY_DURATION));
            Ok(())
        }
    }
}
