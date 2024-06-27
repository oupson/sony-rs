#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum Datatype {
    Ack = 0x1,
    Command1 = 0x0c,
    Command2 = 0x0e,
    Unknown = 0xff,
}

impl From<u8> for Datatype {
    fn from(value: u8) -> Self {
        match value {
            0x1 => Self::Ack,
            0x0c => Self::Command1,
            0x0e => Self::Command2,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum Packet<'a> {
    Ack(u8),
    Command1(u8, PayloadCommand1<'a>),
    Command2(u8),
}

impl<'a> Packet<'a> {
    pub fn seqnum(&self) -> u8 {
        match self {
            Packet::Ack(s) => *s,
            Packet::Command1(s, _) => *s,
            Packet::Command2(s) => *s,
        }
    }

    pub fn write_into(self, buf: &mut [u8]) -> anyhow::Result<usize> {
        buf[0] = 0x3e;
        buf[1] = match self {
            Packet::Ack(_) => 0x01,
            Packet::Command1(_, _) => 0x0c,
            Packet::Command2(_) => 0xe,
        };

        buf[2] = self.seqnum();
        let size = self.write_payload(&mut buf[7..])?;
        buf[3..7].copy_from_slice(&size.to_be_bytes());

        let end = 7 + size as usize;

        let checksum = buf[1..end]
            .iter()
            .fold(0, |acc: u8, x: &u8| acc.wrapping_add(*x));

        buf[end] = checksum;

        buf[end + 1] = 60;

        Ok(end + 2)
    }

    pub fn is_ack(&self) -> bool {
        if let Packet::Ack(_) = self {
            true
        } else {
            false
        }
    }

    fn write_payload(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        match self {
            Packet::Ack(_) => Ok(0),
            Packet::Command1(_, p) => p.write_into(buf),
            Packet::Command2(_) => todo!(),
        }
    }
}

#[derive(Debug)]
pub enum PayloadCommand1<'a> {
    InitRequest,
    InitReply(&'a [u8]),

    FwVersionRequest(&'a [u8]),
    FwVersionReply(&'a [u8]),

    Init2Request(&'a [u8]),
    Init2Reply(&'a [u8]),

    BatteryLevelRequest(&'a [u8]),
    BatteryLevelReply(&'a [u8]),
    BatteryLevelNotify(&'a [u8]),

    AudioCodecRequest(&'a [u8]),
    AudioCodecReply(&'a [u8]),
    AudioCodecNotify(&'a [u8]),

    PowerOff(&'a [u8]),

    SoundPositionOrModeGet(&'a [u8]),
    SoundPositionOrModeRet(&'a [u8]),
    SoundPositionOrModeSet(&'a [u8]),
    SoundPositionOrModeNotify(&'a [u8]),

    EqualizerGet(&'a [u8]),
    EqualizerRet(&'a [u8]),
    EqualizerSet(&'a [u8]),
    EqualizerNotify(&'a [u8]),

    AmbientSoundControlGet,
    AmbientSoundControlRet(AncPayload),
    AmbientSoundControlSet(AncPayload),
    AmbientSoundControlNotify(AncPayload),

    VolumeGet(&'a [u8]),
    VolumeRet(&'a [u8]),
    VolumeSet(&'a [u8]),
    VolumeNotify(&'a [u8]),

    NoiseCancellingOptimizerStart(&'a [u8]),
    NoiseCancellingOptimizerStatus(&'a [u8]),

    NoiseCancellingOptimizerStateGet(&'a [u8]),
    NoiseCancellingOptimizerStateRet(&'a [u8]),
    NoiseCancellingOptimizerStateNotify(&'a [u8]),

    TouchSensorGet(&'a [u8]),
    TouchSensorRet(&'a [u8]),
    TouchSensorSet(&'a [u8]),
    TouchSensorNotify(&'a [u8]),

    AudioUpsamplingGet(&'a [u8]),
    AudioUpsamplingRet(&'a [u8]),
    AudioUpsamplingSet(&'a [u8]),
    AudioUpsamplingNotify(&'a [u8]),

    AutomaticPowerOffButtonModeGet(&'a [u8]),
    AutomaticPowerOffButtonModeRet(&'a [u8]),
    AutomaticPowerOffButtonModeSet(&'a [u8]),
    AutomaticPowerOffButtonModeNotify(&'a [u8]),

    SpeakToChatConfigGet(&'a [u8]),
    SpeakToChatConfigRet(&'a [u8]),
    SpeakToChatConfigSet(&'a [u8]),
    SpeakToChatConfigNotify(&'a [u8]),

    JsonGet(&'a [u8]),
    JsonRet(&'a [u8]),

    SomethingGet(&'a [u8]),
    SomethingRet(&'a [u8]),
}

impl<'a> TryFrom<&'a [u8]> for PayloadCommand1<'a> {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match value[0] {
            0x00 => Ok(Self::InitRequest),
            0x01 => Ok(Self::InitReply(&value[1..])),

            0x04 => Ok(Self::FwVersionRequest(&value[1..])),
            0x05 => Ok(Self::FwVersionReply(&value[1..])),

            0x06 => Ok(Self::Init2Request(&value[1..])),
            0x07 => Ok(Self::Init2Reply(&value[1..])),

            0x10 => Ok(Self::BatteryLevelRequest(&value[1..])),
            0x11 => Ok(Self::BatteryLevelReply(&value[1..])),
            0x13 => Ok(Self::BatteryLevelNotify(&value[1..])),

            0x18 => Ok(Self::AudioCodecRequest(&value[1..])),
            0x19 => Ok(Self::AudioCodecReply(&value[1..])),
            0x1b => Ok(Self::AudioCodecNotify(&value[1..])),

            0x22 => Ok(Self::PowerOff(&value[1..])),

            0x46 => Ok(Self::SoundPositionOrModeGet(&value[1..])),
            0x47 => Ok(Self::SoundPositionOrModeRet(&value[1..])),
            0x48 => Ok(Self::SoundPositionOrModeSet(&value[1..])),
            0x49 => Ok(Self::SoundPositionOrModeNotify(&value[1..])),

            0x56 => Ok(Self::EqualizerGet(&value[1..])),
            0x57 => Ok(Self::EqualizerRet(&value[1..])),
            0x58 => Ok(Self::EqualizerSet(&value[1..])),
            0x59 => Ok(Self::EqualizerNotify(&value[1..])),

            0x66 => Ok(Self::AmbientSoundControlGet),
            0x67 => Ok(Self::AmbientSoundControlRet(AncPayload::try_from(
                &value[1..],
            )?)),
            0x68 => Ok(Self::AmbientSoundControlSet(AncPayload::try_from(
                &value[1..],
            )?)),
            0x69 => Ok(Self::AmbientSoundControlNotify(AncPayload::try_from(
                &value[1..],
            )?)),

            0xa6 => Ok(Self::VolumeGet(&value[1..])),
            0xa7 => Ok(Self::VolumeRet(&value[1..])),
            0xa8 => Ok(Self::VolumeSet(&value[1..])),
            0xa9 => Ok(Self::VolumeNotify(&value[1..])),

            0x84 => Ok(Self::NoiseCancellingOptimizerStart(&value[1..])),
            0x85 => Ok(Self::NoiseCancellingOptimizerStatus(&value[1..])),

            0x86 => Ok(Self::NoiseCancellingOptimizerStateGet(&value[1..])),
            0x87 => Ok(Self::NoiseCancellingOptimizerStateRet(&value[1..])),
            0x89 => Ok(Self::NoiseCancellingOptimizerStateNotify(&value[1..])),

            0xd6 => Ok(Self::TouchSensorGet(&value[1..])),
            0xd7 => Ok(Self::TouchSensorRet(&value[1..])),
            0xd8 => Ok(Self::TouchSensorSet(&value[1..])),
            0xd9 => Ok(Self::TouchSensorNotify(&value[1..])),

            0xe6 => Ok(Self::AudioUpsamplingGet(&value[1..])),
            0xe7 => Ok(Self::AudioUpsamplingRet(&value[1..])),
            0xe8 => Ok(Self::AudioUpsamplingSet(&value[1..])),
            0xe9 => Ok(Self::AudioUpsamplingNotify(&value[1..])),

            0xf6 => Ok(Self::AutomaticPowerOffButtonModeGet(&value[1..])),
            0xf7 => Ok(Self::AutomaticPowerOffButtonModeRet(&value[1..])),
            0xf8 => Ok(Self::AutomaticPowerOffButtonModeSet(&value[1..])),
            0xf9 => Ok(Self::AutomaticPowerOffButtonModeNotify(&value[1..])),

            0xfa => Ok(Self::SpeakToChatConfigGet(&value[1..])),
            0xfb => Ok(Self::SpeakToChatConfigRet(&value[1..])),
            0xfc => Ok(Self::SpeakToChatConfigSet(&value[1..])),
            0xfd => Ok(Self::SpeakToChatConfigNotify(&value[1..])),

            0xc4 => Ok(Self::JsonGet(&value[1..])),
            0xc9 => Ok(Self::JsonRet(&value[1..])),

            0x90 => Ok(Self::SomethingGet(&value[1..])),
            0x91 => Ok(Self::SomethingRet(&value[1..])),
            v => Err(anyhow::format_err!(
                "unknown payload for command1 : {:02x?}",
                v
            )),
        }
    }
}

impl<'a> Payload for PayloadCommand1<'a> {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        match self {
            Self::InitRequest => {
                buf[0] = 0x00;
                buf[1] = 0x00;
                Ok(2)
            }
            Self::InitReply(v) => {
                buf[0] = 0x01;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::FwVersionRequest(v) => {
                buf[0] = 0x04;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::FwVersionReply(v) => {
                buf[0] = 0x05;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::Init2Request(v) => {
                buf[0] = 0x06;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::Init2Reply(v) => {
                buf[0] = 0x07;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::BatteryLevelRequest(v) => {
                buf[0] = 0x10;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::BatteryLevelReply(v) => {
                buf[0] = 0x11;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::BatteryLevelNotify(v) => {
                buf[0] = 0x13;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::AudioCodecRequest(v) => {
                buf[0] = 0x18;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AudioCodecReply(v) => {
                buf[0] = 0x19;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AudioCodecNotify(v) => {
                buf[0] = 0x1b;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::PowerOff(v) => {
                buf[0] = 0x22;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::SoundPositionOrModeGet(v) => {
                buf[0] = 0x46;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SoundPositionOrModeRet(v) => {
                buf[0] = 0x47;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SoundPositionOrModeSet(v) => {
                buf[0] = 0x48;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SoundPositionOrModeNotify(v) => {
                buf[0] = 0x49;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::EqualizerGet(v) => {
                buf[0] = 0x56;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::EqualizerRet(v) => {
                buf[0] = 0x57;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::EqualizerSet(v) => {
                buf[0] = 0x58;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::EqualizerNotify(v) => {
                buf[0] = 0x59;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::AmbientSoundControlGet => {
                buf[0] = 0x66;
                let len = GetAnc.write_into(&mut buf[1..])?;
                Ok((len + 1) as u32)
            }

            Self::AmbientSoundControlRet(v) => {
                buf[0] = 0x67;
                let len = v.write_into(&mut buf[1..])?;
                Ok((len + 1) as u32)
            }
            Self::AmbientSoundControlSet(v) => {
                buf[0] = 0x68;
                let len = v.write_into(&mut buf[1..])?;
                Ok((len + 1) as u32)
            }
            Self::AmbientSoundControlNotify(v) => {
                buf[0] = 0x69;
                let len = v.write_into(&mut buf[1..])?;
                Ok((len + 1) as u32)
            }

            Self::VolumeGet(v) => {
                buf[0] = 0xa6;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::VolumeRet(v) => {
                buf[0] = 0xa7;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::VolumeSet(v) => {
                buf[0] = 0xa8;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::VolumeNotify(v) => {
                buf[0] = 0xa9;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::NoiseCancellingOptimizerStart(v) => {
                buf[0] = 0x84;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::NoiseCancellingOptimizerStatus(v) => {
                buf[0] = 0x85;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::NoiseCancellingOptimizerStateGet(v) => {
                buf[0] = 0x86;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::NoiseCancellingOptimizerStateRet(v) => {
                buf[0] = 0x87;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::NoiseCancellingOptimizerStateNotify(v) => {
                buf[0] = 0x89;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::TouchSensorGet(v) => {
                buf[0] = 0xd6;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::TouchSensorRet(v) => {
                buf[0] = 0xd7;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::TouchSensorSet(v) => {
                buf[0] = 0xd8;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::TouchSensorNotify(v) => {
                buf[0] = 0xd9;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::AudioUpsamplingGet(v) => {
                buf[0] = 0xe6;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AudioUpsamplingRet(v) => {
                buf[0] = 0xe7;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AudioUpsamplingSet(v) => {
                buf[0] = 0xe8;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AudioUpsamplingNotify(v) => {
                buf[0] = 0xe9;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::AutomaticPowerOffButtonModeGet(v) => {
                buf[0] = 0xf6;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AutomaticPowerOffButtonModeRet(v) => {
                buf[0] = 0xf7;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AutomaticPowerOffButtonModeSet(v) => {
                buf[0] = 0xf8;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::AutomaticPowerOffButtonModeNotify(v) => {
                buf[0] = 0xf9;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::SpeakToChatConfigGet(v) => {
                buf[0] = 0xfa;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SpeakToChatConfigRet(v) => {
                buf[0] = 0xfb;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SpeakToChatConfigSet(v) => {
                buf[0] = 0xfc;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SpeakToChatConfigNotify(v) => {
                buf[0] = 0xfd;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::JsonGet(v) => {
                buf[0] = 0xc4;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::JsonRet(v) => {
                buf[0] = 0xc9;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }

            Self::SomethingGet(v) => {
                buf[0] = 0x90;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
            Self::SomethingRet(v) => {
                buf[0] = 0x91;
                buf[1..1 + v.len()].copy_from_slice(v);
                Ok((v.len() + 1) as u32)
            }
        }
    }
}

#[derive(Debug)]
pub enum AllPayload<'a> {
    Empty,
    Unknown(&'a [u8]),
}

impl<'a> Payload for AllPayload<'a> {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        match self {
            Self::Empty => Ok(0),
            AllPayload::Unknown(b) => b.write_into(buf),
        }
    }
}

impl<'a> From<&'a [u8]> for AllPayload<'a> {
    fn from(value: &'a [u8]) -> Self {
        match value[0] {
            //  (PayloadTypeCommand1::AmbientSoundControlRet as u8) => Self::Unknown(value),
            _ => Self::Unknown(value),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Packet<'a> {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        // TODO HEADER / END / CHECKSUM
        //
        //
        let seqnum = value[2];

        return match value[1] {
            0x1 => Ok(Packet::Ack(seqnum)),
            0x0c => {
                let packet_size = u32::from_be_bytes(value[3..][0..4].try_into()?); // TODO

                let payload_raw = &value[7..7 + packet_size as usize];

                let payload = PayloadCommand1::try_from(payload_raw)?;

                Ok(Packet::Command1(seqnum, payload))
            }
            0x0e => Ok(Packet::Command2(seqnum)),
            _ => todo!(),
        };
    }
}

#[derive(Debug)]
pub struct AncPayload {
    pub anc_mode: AncMode,
    pub focus_on_voice: bool,
    pub ambiant_level: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AncMode {
    Off,
    AmbiantMode,
    On,
    Wind,
}

pub trait Payload {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32>;
}

impl Payload for AncPayload {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        // TODO invalid buffer size
        buf[0] = 0x02;
        buf[1] = if self.anc_mode == AncMode::Off {
            0x00
        } else {
            0x11
        };
        buf[2] = 0x02;
        buf[3] = match self.anc_mode {
            AncMode::Off | AncMode::AmbiantMode => 0,
            AncMode::On => 0x02,
            AncMode::Wind => 0x01,
        };
        buf[4] = 0x01;
        buf[5] = if self.focus_on_voice { 0x01 } else { 0x00 };
        buf[6] = match self.anc_mode {
            AncMode::Off | AncMode::AmbiantMode => self.ambiant_level,
            AncMode::On | AncMode::Wind => 0x1,
        };
        Ok(7)
    }
}

impl Payload for &[u8] {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        buf[0..self.len()].copy_from_slice(self);
        Ok(self.len() as u32)
    }
}

impl Payload for () {
    fn write_into(&self, _: &mut [u8]) -> anyhow::Result<u32> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct GetAnc;

impl Payload for GetAnc {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        buf[0] = 0x02;

        Ok(1)
    }
}

impl TryFrom<&[u8]> for AncPayload {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        assert_eq!(7, value.len());
        let mode = match value[1] {
            0x00 => AncMode::Off,
            0x01 => {
                if value[2] == 0x00 {
                    // Only ANC  and Ambient Sound supported?
                    if value[3] == 0x00 {
                        AncMode::AmbiantMode
                    } else if value[3] == 0x01 {
                        AncMode::On
                    } else {
                        unimplemented!()
                    }
                } else if value[2] == 0x02 {
                    // Supports wind noise reduction
                    if value[3] == 0x00 {
                        AncMode::AmbiantMode
                    } else if value[3] == 0x01 {
                        AncMode::Wind
                    } else if value[3] == 0x02 {
                        AncMode::On
                    } else {
                        unimplemented!()
                    }
                } else {
                    unimplemented!()
                }
            }
            _ => unimplemented!(),
        };

        let focus_on_voice = value[5] == 0x01;

        let ambiant_level = value[6];
        Ok(Self {
            anc_mode: mode,
            focus_on_voice,
            ambiant_level,
        })
    }
}
