use std::array::TryFromSliceError;

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
    Case {
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
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let battery_type = BatteryType::try_from(value[0])?;
        match battery_type {
            BatteryType::Single => {
                let level = value[1];
                let is_charging = value[2] == 1;
                Ok(BatteryState::Single { level, is_charging })
            }
            BatteryType::Case => {
                let level = value[1];
                let is_charging = value[2] == 1;
                Ok(BatteryState::Case { level, is_charging })
            }
            BatteryType::Dual => {
                if value[1] == 0 {
                    Ok(BatteryState::Single {
                        level: value[3],
                        is_charging: value[4] == 1,
                    })
                } else if value[3] == 0 {
                    Ok(BatteryState::Single {
                        level: value[1],
                        is_charging: value[2] == 1,
                    })
                } else {
                    Ok(BatteryState::Dual {
                        level_left: value[1],
                        is_left_charging: value[2] == 1,
                        level_right: value[3],
                        is_right_charging: value[4] == 1,
                    })
                }
            }
        }
    }
}

impl TryFrom<u8> for BatteryType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Single),
            1 => Ok(Self::Dual),
            2 => Ok(Self::Case),
            value => Err(crate::Error::InvalidValueForEnum {
                what: "battery type",
                value,
            }),
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

    pub fn write_into(self, buf: &mut [u8]) -> crate::Result<usize> {
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

    fn write_payload(&self, buf: &mut [u8]) -> crate::Result<u32> {
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
    type Error = crate::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match value[0] {
            0x00 => Ok(Self::InitRequest),
            0x01 => {
                assert!(value.len() > 3);
                Ok(Self::InitReply([value[1], value[2], value[3]]))
            }

            0x04 => Err(crate::Error::NotImplemented("Self::FwVersionRequest")),
            0x05 => Err(crate::Error::NotImplemented("Self::FwVersionReply")),

            0x06 => Err(crate::Error::NotImplemented("Self::Init2Request")),
            0x07 => Err(crate::Error::NotImplemented("Self::Init2Reply")),

            0x10 => Ok(PayloadCommand1::BatteryLevelRequest(BatteryType::try_from(
                value[1],
            )?)),
            0x11 => Ok(PayloadCommand1::BatteryLevelReply(BatteryState::try_from(
                &value[1..],
            )?)),
            0x13 => Ok(PayloadCommand1::BatteryLevelNotify(BatteryState::try_from(
                &value[1..],
            )?)),

            0x18 => Err(crate::Error::NotImplemented("Self::AudioCodecRequest")),
            0x19 => Err(crate::Error::NotImplemented("Self::AudioCodecReply")),
            0x1b => Err(crate::Error::NotImplemented("Self::AudioCodecNotify")),

            0x22 => Err(crate::Error::NotImplemented("Self::PowerOff")),

            0x46 => Err(crate::Error::NotImplemented("Self::SoundPositionOrModeGet")),
            0x47 => Err(crate::Error::NotImplemented("Self::SoundPositionOrModeRet")),
            0x48 => Err(crate::Error::NotImplemented("Self::SoundPositionOrModeSet")),
            0x49 => Err(crate::Error::NotImplemented(
                "Self::SoundPositionOrModeNotify",
            )),

            0x56 => Err(crate::Error::NotImplemented("Self::EqualizerGet")),
            0x57 => Err(crate::Error::NotImplemented("Self::EqualizerRet")),
            0x58 => Err(crate::Error::NotImplemented("Self::EqualizerSet")),
            0x59 => Err(crate::Error::NotImplemented("Self::EqualizerNotify")),

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

            0xa6 => Err(crate::Error::NotImplemented("Self::VolumeGet")),
            0xa7 => Err(crate::Error::NotImplemented("Self::VolumeRet")),
            0xa8 => Err(crate::Error::NotImplemented("Self::VolumeSet")),
            0xa9 => Err(crate::Error::NotImplemented("Self::VolumeNotify")),

            0x84 => Err(crate::Error::NotImplemented(
                "Self::NoiseCancellingOptimizerStart",
            )),
            0x85 => Err(crate::Error::NotImplemented(
                "Self::NoiseCancellingOptimizerStatus",
            )),

            0x86 => Err(crate::Error::NotImplemented(
                "Self::NoiseCancellingOptimizerStateGet",
            )),
            0x87 => Err(crate::Error::NotImplemented(
                "Self::NoiseCancellingOptimizerStateRet",
            )),
            0x89 => Err(crate::Error::NotImplemented(
                "Self::NoiseCancellingOptimizerStateNotify",
            )),

            0xd6 => Err(crate::Error::NotImplemented("Self::TouchSensorGet")),
            0xd7 => Err(crate::Error::NotImplemented("Self::TouchSensorRet")),
            0xd8 => Err(crate::Error::NotImplemented("Self::TouchSensorSet")),
            0xd9 => Err(crate::Error::NotImplemented("Self::TouchSensorNotify")),

            0xe6 => Err(crate::Error::NotImplemented("Self::AudioUpsamplingGet")),
            0xe7 => Err(crate::Error::NotImplemented("Self::AudioUpsamplingRet")),
            0xe8 => Err(crate::Error::NotImplemented("Self::AudioUpsamplingSet")),
            0xe9 => Err(crate::Error::NotImplemented("Self::AudioUpsamplingNotify")),

            0xf6 => Err(crate::Error::NotImplemented(
                "Self::AutomaticPowerOffButtonModeGet",
            )),
            0xf7 => Err(crate::Error::NotImplemented(
                "Self::AutomaticPowerOffButtonModeRet",
            )),
            0xf8 => Err(crate::Error::NotImplemented(
                "Self::AutomaticPowerOffButtonModeSset",
            )),
            0xf9 => Err(crate::Error::NotImplemented(
                "Self::AutomaticPowerOffButtonModeNotify",
            )),

            0xfa => Err(crate::Error::NotImplemented("Self::SpeakToChatConfigGet")),
            0xfb => Err(crate::Error::NotImplemented("Self::SpeakToChatConfigRet")),
            0xfc => Err(crate::Error::NotImplemented("Self::SpeakToChatConfigSet")),
            0xfd => Err(crate::Error::NotImplemented(
                "Self::SpeakToChatConfigNotify",
            )),

            0xc4 => Err(crate::Error::NotImplemented("Self::JsonGet")),
            0xc9 => Err(crate::Error::NotImplemented("Self::JsonRet")),

            0x90 => Err(crate::Error::NotImplemented("Self::SomethingGet")),
            0x91 => Err(crate::Error::NotImplemented("Self::SomethingRet")),
            v => Err(crate::Error::UnknownPayloadType(v)),
        }
    }
}

impl<'a> Payload for PayloadCommand1 {
    fn write_into(&self, buf: &mut [u8]) -> crate::Result<u32> {
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
            Self::FwVersionRequest => Err(crate::Error::NotImplemented("0x04")),
            Self::FwVersionReply => Err(crate::Error::NotImplemented("0x05")),
            Self::Init2Request => Err(crate::Error::NotImplemented("0x06")),
            Self::Init2Reply => Err(crate::Error::NotImplemented("0x07")),
            Self::BatteryLevelRequest(b) => {
                buf[0] = 0x10;
                buf[1] = *b as u8;
                Ok(2)
            }
            Self::BatteryLevelReply(_state) => Err(crate::Error::NotImplemented("0x11")),
            Self::BatteryLevelNotify(_state) => Err(crate::Error::NotImplemented("0x13")),
            Self::AudioCodecRequest => Err(crate::Error::NotImplemented("0x18")),
            Self::AudioCodecReply => Err(crate::Error::NotImplemented("0x19")),
            Self::AudioCodecNotify => Err(crate::Error::NotImplemented("0x1b")),
            Self::PowerOff => Err(crate::Error::NotImplemented("0x22")),
            Self::SoundPositionOrModeGet => Err(crate::Error::NotImplemented("0x46")),
            Self::SoundPositionOrModeRet => Err(crate::Error::NotImplemented("0x47")),
            Self::SoundPositionOrModeSet => Err(crate::Error::NotImplemented("0x48")),
            Self::SoundPositionOrModeNotify => Err(crate::Error::NotImplemented("0x49")),
            Self::EqualizerGet => Err(crate::Error::NotImplemented("0x56")),
            Self::EqualizerRet => Err(crate::Error::NotImplemented("0x57")),
            Self::EqualizerSet => Err(crate::Error::NotImplemented("0x58")),
            Self::EqualizerNotify => Err(crate::Error::NotImplemented("0x59")),

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
            Self::VolumeGet => Err(crate::Error::NotImplemented("0xa6")),
            Self::VolumeRet => Err(crate::Error::NotImplemented("0xa7")),
            Self::VolumeSet => Err(crate::Error::NotImplemented("0xa8")),
            Self::VolumeNotify => Err(crate::Error::NotImplemented("0xa9")),
            Self::NoiseCancellingOptimizerStart => Err(crate::Error::NotImplemented("0x84")),
            Self::NoiseCancellingOptimizerStatus => Err(crate::Error::NotImplemented("0x85")),
            Self::NoiseCancellingOptimizerStateGet => Err(crate::Error::NotImplemented("0x86")),
            Self::NoiseCancellingOptimizerStateRet => Err(crate::Error::NotImplemented("0x87")),
            Self::NoiseCancellingOptimizerStateNotify => Err(crate::Error::NotImplemented("0x89")),
            Self::TouchSensorGet => Err(crate::Error::NotImplemented("0xd6")),
            Self::TouchSensorRet => Err(crate::Error::NotImplemented("0xd7")),
            Self::TouchSensorSet => Err(crate::Error::NotImplemented("0xd8")),
            Self::TouchSensorNotify => Err(crate::Error::NotImplemented("0xd9")),
            Self::AudioUpsamplingGet => Err(crate::Error::NotImplemented("0xe6")),
            Self::AudioUpsamplingRet => Err(crate::Error::NotImplemented("0xe7")),
            Self::AudioUpsamplingSet => Err(crate::Error::NotImplemented("0xe8")),
            Self::AudioUpsamplingNotify => Err(crate::Error::NotImplemented("0xe9")),
            Self::AutomaticPowerOffButtonModeGet => Err(crate::Error::NotImplemented("0xf6")),
            Self::AutomaticPowerOffButtonModeRet => Err(crate::Error::NotImplemented("0xf7")),
            Self::AutomaticPowerOffButtonModeSet => Err(crate::Error::NotImplemented("0xf8")),
            Self::AutomaticPowerOffButtonModeNotify => Err(crate::Error::NotImplemented("0xf9")),
            Self::SpeakToChatConfigGet => Err(crate::Error::NotImplemented("0xfa")),
            Self::SpeakToChatConfigRet => Err(crate::Error::NotImplemented("0xfb")),
            Self::SpeakToChatConfigSet => Err(crate::Error::NotImplemented("0xfc")),
            Self::SpeakToChatConfigNotify => Err(crate::Error::NotImplemented("0xfd")),
            Self::JsonGet => Err(crate::Error::NotImplemented("0xc4")),
            Self::JsonRet => Err(crate::Error::NotImplemented("0xc9")),
            Self::SomethingGet => Err(crate::Error::NotImplemented("0x90")),
            Self::SomethingRet => Err(crate::Error::NotImplemented("0x91")),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Packet {
    type Error = crate::TryFromPacketError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        // TODO HEADER / END / CHECKSUM
        //
        //
        let seqnum = value[2];

        let content = match value[1] {
            0x1 => Ok(PacketContent::Ack),
            0x0c => {
                let packet_size = u32::from_be_bytes(
                    value[3..][0..4]
                        .try_into()
                        .map_err(|e: TryFromSliceError| Into::<crate::Error>::into(e))
                        .map_err(|error| crate::TryFromPacketError { seqnum, error })?,
                ); // TODO

                let payload_raw = &value[7..7 + packet_size as usize];

                let payload = PayloadCommand1::try_from(payload_raw);

                match payload {
                    Ok(p) => Ok(PacketContent::Command1(p)),
                    Err(error) => Err(crate::TryFromPacketError { seqnum, error }),
                }
            }
            0x0e => Ok(PacketContent::Command2),
            _ => todo!(),
        }?;

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
    fn write_into(&self, buf: &mut [u8]) -> crate::Result<u32>;
}

impl Payload for AncPayload {
    fn write_into(&self, buf: &mut [u8]) -> crate::Result<u32> {
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
    fn write_into(&self, buf: &mut [u8]) -> crate::Result<u32> {
        buf[0..self.len()].copy_from_slice(self);
        Ok(self.len() as u32)
    }
}

impl Payload for () {
    fn write_into(&self, _: &mut [u8]) -> crate::Result<u32> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct GetAnc;

impl Payload for GetAnc {
    fn write_into(&self, buf: &mut [u8]) -> crate::Result<u32> {
        buf[0] = 0x02;

        Ok(1)
    }
}

impl TryFrom<&[u8]> for AncPayload {
    type Error = crate::Error;

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
