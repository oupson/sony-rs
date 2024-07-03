use std::backtrace::BacktraceStatus;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BatteryType {
    Single = 0,
    Dual = 1,
    Case = 2,
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BatteryState {
    Single {
        level: u8,
        is_charging: bool,
    },
    Dual {
        level_left: u8,
        is_left_charging: bool,
        level_right: u8,
        is_right_charging: bool,
    },
}

impl TryFrom<&[u8]> for BatteryState {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let battery_type = BatteryType::try_from(value[0])?;
        match battery_type {
            BatteryType::Single | BatteryType::Case => {
                let level = value[1];
                let is_charging = value[2] == 1;
                Ok(BatteryState::Single { level, is_charging })
            }
            BatteryType::Dual => Ok(BatteryState::Dual {
                level_left: value[1],
                is_left_charging: value[2] == 1,
                level_right: value[3],
                is_right_charging: value[4] == 1,
            }),
        }
    }
}

impl TryFrom<u8> for BatteryType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Single),
            1 => Ok(Self::Dual),
            2 => Ok(Self::Case),
            value => Err(anyhow::format_err!("invalid battery type : {:02x?}", value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Packet {
    pub seqnum: u8,
    pub content: PacketContent,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PacketContent {
    Ack,
    Command1(PayloadCommand1),
    Command2,
}

impl Packet {
    pub fn new(seqnum: u8, content: PacketContent) -> Self {
        Self { seqnum, content }
    }

    pub fn seqnum(&self) -> u8 {
        self.seqnum
    }

    pub fn write_into(self, buf: &mut [u8]) -> anyhow::Result<usize> {
        buf[0] = 0x3e;
        buf[1] = match self.content {
            PacketContent::Ack => 0x01,
            PacketContent::Command1(_) => 0x0c,
            PacketContent::Command2 => 0xe,
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
        PacketContent::Ack == self.content
    }

    fn write_payload(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        match &self.content {
            PacketContent::Ack => Ok(0),
            PacketContent::Command1(p) => p.write_into(buf),
            PacketContent::Command2 => todo!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
pub enum PayloadCommand1 {
    InitRequest,
    InitReply([u8; 3]),

    FwVersionRequest,
    FwVersionReply,

    Init2Request,
    Init2Reply,

    BatteryLevelRequest(BatteryType),
    BatteryLevelReply(BatteryState),
    BatteryLevelNotify(BatteryState),

    AudioCodecRequest,
    AudioCodecReply,
    AudioCodecNotify,

    PowerOff,

    SoundPositionOrModeGet,
    SoundPositionOrModeRet,
    SoundPositionOrModeSet,
    SoundPositionOrModeNotify,

    EqualizerGet,
    EqualizerRet,
    EqualizerSet,
    EqualizerNotify,

    AmbientSoundControlGet,
    AmbientSoundControlRet(AncPayload),
    AmbientSoundControlSet(AncPayload),
    AmbientSoundControlNotify(AncPayload),

    VolumeGet,
    VolumeRet,
    VolumeSet,
    VolumeNotify,

    NoiseCancellingOptimizerStart,
    NoiseCancellingOptimizerStatus,

    NoiseCancellingOptimizerStateGet,
    NoiseCancellingOptimizerStateRet,
    NoiseCancellingOptimizerStateNotify,

    TouchSensorGet,
    TouchSensorRet,
    TouchSensorSet,
    TouchSensorNotify,

    AudioUpsamplingGet,
    AudioUpsamplingRet,
    AudioUpsamplingSet,
    AudioUpsamplingNotify,

    AutomaticPowerOffButtonModeGet,
    AutomaticPowerOffButtonModeRet,
    AutomaticPowerOffButtonModeSet,
    AutomaticPowerOffButtonModeNotify,

    SpeakToChatConfigGet,
    SpeakToChatConfigRet,
    SpeakToChatConfigSet,
    SpeakToChatConfigNotify,

    JsonGet,
    JsonRet,

    SomethingGet,
    SomethingRet,
}

impl<'a> TryFrom<&'a [u8]> for PayloadCommand1 {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match value[0] {
            0x00 => Ok(Self::InitRequest),
            0x01 => {
                assert!(value.len() > 3);
                Ok(Self::InitReply([value[1], value[2], value[3]]))
            }

            0x04 => todo!("Self::FwVersionRequest"),
            0x05 => todo!("Self::FwVersionReply"),

            0x06 => todo!("Self::Init2Request"),
            0x07 => todo!("Self::Init2Reply"),

            0x10 => Ok(PayloadCommand1::BatteryLevelRequest(BatteryType::try_from(
                value[1],
            )?)),
            0x11 => Ok(PayloadCommand1::BatteryLevelReply(BatteryState::try_from(
                &value[1..],
            )?)),
            0x13 => Ok(PayloadCommand1::BatteryLevelNotify(BatteryState::try_from(
                &value[1..],
            )?)),

            0x18 => todo!("Self::AudioCodecRequest"),
            0x19 => todo!("Self::AudioCodecReply"),
            0x1b => todo!("Self::AudioCodecNotify"),

            0x22 => todo!("Self::PowerOff"),

            0x46 => todo!("Self::SoundPositionOrModeGet"),
            0x47 => todo!("Self::SoundPositionOrModeRet"),
            0x48 => todo!("Self::SoundPositionOrModeSet"),
            0x49 => todo!("Self::SoundPositionOrModeNotify"),

            0x56 => todo!("Self::EqualizerGet"),
            0x57 => todo!("Self::EqualizerRet"),
            0x58 => todo!("Self::EqualizerSet"),
            0x59 => todo!("Self::EqualizerNotify"),

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

            0xa6 => todo!("Self::VolumeGet"),
            0xa7 => todo!("Self::VolumeRet"),
            0xa8 => todo!("Self::VolumeSet"),
            0xa9 => todo!("Self::VolumeNotify"),

            0x84 => todo!("Self::NoiseCancellingOptimizerStart"),
            0x85 => todo!("Self::NoiseCancellingOptimizerStatus"),

            0x86 => todo!("Self::NoiseCancellingOptimizerStateGet"),
            0x87 => todo!("Self::NoiseCancellingOptimizerStateRet"),
            0x89 => todo!("Self::NoiseCancellingOptimizerStateNotify"),

            0xd6 => todo!("Self::TouchSensorGet"),
            0xd7 => todo!("Self::TouchSensorRet"),
            0xd8 => todo!("Self::TouchSensorSet"),
            0xd9 => todo!("Self::TouchSensorNotify"),

            0xe6 => todo!("Self::AudioUpsamplingGet"),
            0xe7 => todo!("Self::AudioUpsamplingRet"),
            0xe8 => todo!("Self::AudioUpsamplingSet"),
            0xe9 => todo!("Self::AudioUpsamplingNotify"),

            0xf6 => todo!("Self::AutomaticPowerOffButtonModeGet"),
            0xf7 => todo!("Self::AutomaticPowerOffButtonModeRet"),
            0xf8 => todo!("Self::AutomaticPowerOffButtonModeSet"),
            0xf9 => todo!("Self::AutomaticPowerOffButtonModeNotify"),

            0xfa => todo!("Self::SpeakToChatConfigGet"),
            0xfb => todo!("Self::SpeakToChatConfigRet"),
            0xfc => todo!("Self::SpeakToChatConfigSet"),
            0xfd => todo!("Self::SpeakToChatConfigNotify"),

            0xc4 => todo!("Self::JsonGet"),
            0xc9 => todo!("Self::JsonRet"),

            0x90 => todo!("Self::SomethingGet"),
            0x91 => todo!("Self::SomethingRet"),
            v => Err(anyhow::format_err!(
                "unknown payload for command1 : {:02x?}",
                v
            )),
        }
    }
}

impl<'a> Payload for PayloadCommand1 {
    fn write_into(&self, buf: &mut [u8]) -> anyhow::Result<u32> {
        match self {
            Self::InitRequest => {
                buf[0] = 0x00;
                buf[1] = 0x00;
                Ok(2)
            }
            Self::InitReply(b) => {
                buf[0] = 0x01;
                buf[1..4].copy_from_slice(b);
                Ok(4)
            }
            Self::FwVersionRequest => todo!("0x04"),
            Self::FwVersionReply => todo!("0x05"),
            Self::Init2Request => todo!("0x06"),
            Self::Init2Reply => todo!("0x07"),
            Self::BatteryLevelRequest(b) => {
                buf[0] = 0x10;
                buf[1] = *b as u8;
                Ok(2)
            }
            Self::BatteryLevelReply(state) => todo!("0x11"),
            Self::BatteryLevelNotify(state) => todo!("0x13"),
            Self::AudioCodecRequest => todo!("0x18"),
            Self::AudioCodecReply => todo!("0x19"),
            Self::AudioCodecNotify => todo!("0x1b"),
            Self::PowerOff => todo!("0x22"),
            Self::SoundPositionOrModeGet => todo!("0x46"),
            Self::SoundPositionOrModeRet => todo!("0x47"),
            Self::SoundPositionOrModeSet => todo!("0x48"),
            Self::SoundPositionOrModeNotify => todo!("0x49"),
            Self::EqualizerGet => todo!("0x56"),
            Self::EqualizerRet => todo!("0x57"),
            Self::EqualizerSet => todo!("0x58"),
            Self::EqualizerNotify => todo!("0x59"),

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
            Self::VolumeGet => todo!("0xa6"),
            Self::VolumeRet => todo!("0xa7"),
            Self::VolumeSet => todo!("0xa8"),
            Self::VolumeNotify => todo!("0xa9"),
            Self::NoiseCancellingOptimizerStart => todo!("0x84"),
            Self::NoiseCancellingOptimizerStatus => todo!("0x85"),
            Self::NoiseCancellingOptimizerStateGet => todo!("0x86"),
            Self::NoiseCancellingOptimizerStateRet => todo!("0x87"),
            Self::NoiseCancellingOptimizerStateNotify => todo!("0x89"),
            Self::TouchSensorGet => todo!("0xd6"),
            Self::TouchSensorRet => todo!("0xd7"),
            Self::TouchSensorSet => todo!("0xd8"),
            Self::TouchSensorNotify => todo!("0xd9"),
            Self::AudioUpsamplingGet => todo!("0xe6"),
            Self::AudioUpsamplingRet => todo!("0xe7"),
            Self::AudioUpsamplingSet => todo!("0xe8"),
            Self::AudioUpsamplingNotify => todo!("0xe9"),
            Self::AutomaticPowerOffButtonModeGet => todo!("0xf6"),
            Self::AutomaticPowerOffButtonModeRet => todo!("0xf7"),
            Self::AutomaticPowerOffButtonModeSet => todo!("0xf8"),
            Self::AutomaticPowerOffButtonModeNotify => todo!("0xf9"),
            Self::SpeakToChatConfigGet => todo!("0xfa"),
            Self::SpeakToChatConfigRet => todo!("0xfb"),
            Self::SpeakToChatConfigSet => todo!("0xfc"),
            Self::SpeakToChatConfigNotify => todo!("0xfd"),
            Self::JsonGet => todo!("0xc4"),
            Self::JsonRet => todo!("0xc9"),
            Self::SomethingGet => todo!("0x90"),
            Self::SomethingRet => todo!("0x91"),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Packet {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        // TODO HEADER / END / CHECKSUM
        //
        //
        let seqnum = value[2];

        let content = match value[1] {
            0x1 => PacketContent::Ack,
            0x0c => {
                let packet_size = u32::from_be_bytes(value[3..][0..4].try_into()?); // TODO

                let payload_raw = &value[7..7 + packet_size as usize];

                let payload = PayloadCommand1::try_from(payload_raw)?;

                PacketContent::Command1(payload)
            }
            0x0e => PacketContent::Command2,
            _ => todo!(),
        };

        Ok(Packet { seqnum, content })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AncPayload {
    pub anc_mode: AncMode,
    pub focus_on_voice: bool,
    pub ambiant_level: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
