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
pub struct Packet<P>
where
    P: Payload,
{
    pub data_type: Datatype,
    pub seqnum: u8,
    pub payload: P,
}

impl<P> Packet<P>
where
    P: Payload,
{
    pub fn write_into(self, buf: &mut [u8]) -> anyhow::Result<usize> {
        // TODO invalid buffer size
        buf[0] = 0x3e;
        buf[1] = self.data_type as u8;
        buf[2] = self.seqnum;
        let size = self.payload.write_into(&mut buf[7..])?;
        buf[3..7].copy_from_slice(&size.to_be_bytes());

        let end = 7 + size as usize;

        let checksum = buf[1..end]
            .iter()
            .fold(0, |acc: u8, x: &u8| acc.wrapping_add(*x));

        buf[end] = checksum;

        buf[end + 1] = 60;

        Ok(end + 2)
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
            _ => Self::Unknown(value),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Packet<AllPayload<'a>> {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        // TODO HEADER / END / CHECKSUM
        let command = Datatype::from(value[1]);
        let seqnum = value[2];

        let packet_size = u32::from_be_bytes(value[3..][0..4].try_into()?); // TODO

        let payload_raw = &value[7..7 + packet_size as usize];

        Ok(Packet {
            data_type: command,
            seqnum,
            payload: if command != Datatype::Ack {
                AllPayload::from(payload_raw)
            } else {
                AllPayload::Empty
            },
        })
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum PayloadTypeCommand1 {
    InitRequest = 0x00,
    InitReply = 0x01,

    FwVersionRequest = 0x04,
    FwVersionReply = 0x05,

    Init2Request = 0x06,
    Init2Reply = 0x07,

    BatteryLevelRequest = 0x10,
    BatteryLevelReply = 0x11,
    BatteryLevelNotify = 0x13,

    AudioCodecRequest = 0x18,
    AudioCodecReply = 0x19,
    AudioCodecNotify = 0x1b,

    PowerOff = 0x22,

    SoundPositionOrModeGet = 0x46,
    SoundPositionOrModeRet = 0x47,
    SoundPositionOrModeSet = 0x48,
    SoundPositionOrModeNotify = 0x49,

    EqualizerGet = 0x56,
    EqualizerRet = 0x57,
    EqualizerSet = 0x58,
    EqualizerNotify = 0x59,

    AmbientSoundControlGet = 0x66,
    AmbientSoundControlRet = 0x67,
    AmbientSoundControlSet = 0x68,
    AmbientSoundControlNotify = 0x69,

    VolumeGet = 0xa6,
    VolumeRet = 0xa7,
    VolumeSet = 0xa8,
    VolumeNotify = 0xa9,

    NoiseCancellingOptimizerStart = 0x84,
    NoiseCancellingOptimizerStatus = 0x85,

    NoiseCancellingOptimizerStateGet = 0x86,
    NoiseCancellingOptimizerStateRet = 0x87,
    NoiseCancellingOptimizerStateNotify = 0x89,

    TouchSensorGet = 0xd6,
    TouchSensorRet = 0xd7,
    TouchSensorSet = 0xd8,
    TouchSensorNotify = 0xd9,

    AudioUpsamplingGet = 0xe6,
    AudioUpsamplingRet = 0xe7,
    AudioUpsamplingSet = 0xe8,
    AudioUpsamplingNotify = 0xe9,

    AutomaticPowerOffButtonModeGet = 0xf6,
    AutomaticPowerOffButtonModeRet = 0xf7,
    AutomaticPowerOffButtonModeSet = 0xf8,
    AutomaticPowerOffButtonModeNotify = 0xf9,

    SpeakToChatConfigGet = 0xfa,
    SpeakToChatConfigRet = 0xfb,
    SpeakToChatConfigSet = 0xfc,
    SpeakToChatConfigNotify = 0xfd,

    JsonGet = 0xc4,
    JsonRet = 0xc9,

    SomethingGet = 0x90,
    SomethingRet = 0x91,

    Unknown = 0xff,
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
        buf[0] = PayloadTypeCommand1::AmbientSoundControlSet as u8;
        buf[1] = 0x02;
        buf[2] = if self.anc_mode == AncMode::Off {
            0x00
        } else {
            0x11
        };
        buf[3] = 0x02;
        buf[4] = match self.anc_mode {
            AncMode::Off | AncMode::AmbiantMode => 0,
            AncMode::On => 0x02,
            AncMode::Wind => 0x01,
        };
        buf[5] = 0x01;
        buf[6] = if self.focus_on_voice { 0x01 } else { 0x00 };
        buf[7] = match self.anc_mode {
            AncMode::Off | AncMode::AmbiantMode => self.ambiant_level,
            AncMode::On | AncMode::Wind => 0x1,
        };
        Ok(8)
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
        buf[0] = PayloadTypeCommand1::AmbientSoundControlGet as u8;
        buf[1] = 0x02;

        Ok(2)
    }
}
