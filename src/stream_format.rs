use crate::error::AudioUnitError;
use crate::format::{AudioFormat, LinearPcmFlags};
use crate::{CAError, SampleFormat};

//
pub struct StreamFormat {
    pub(crate) asbd: sys::AudioStreamBasicDescription,
}

impl StreamFormat {
    pub fn new(
        sample_rate: f64,
        sample_format: SampleFormat,
        flags: LinearPcmFlags,
        channels: usize,
    ) -> Self {
        let (format, format_flags) =
            AudioFormat::LinearPCM(flags | LinearPcmFlags::IS_PACKED).as_format_and_flag();

        //  TODO: What's going on here?
        let format_flags = format_flags.unwrap_or(::std::u32::MAX - 2147483647);

        let non_interleaved = flags.contains(LinearPcmFlags::IS_NON_INTERLEAVED);

        let channels = channels as u32;

        let bytes_per_frame = if non_interleaved {
            sample_format.size_in_bytes() as u32
        } else {
            sample_format.size_in_bytes() as u32 * channels
        };

        const FRAMES_PER_PACKET: u32 = 1;

        let bytes_per_packet = bytes_per_frame * FRAMES_PER_PACKET;
        let bits_per_channel = sample_format.size_in_bits();

        let asbd = sys::AudioStreamBasicDescription {
            mSampleRate: sample_rate,
            mFormatID: format,
            mFormatFlags: format_flags,
            mBytesPerPacket: bytes_per_packet,
            mFramesPerPacket: FRAMES_PER_PACKET,
            mBytesPerFrame: bytes_per_frame,
            mChannelsPerFrame: channels,
            mBitsPerChannel: bits_per_channel,
            mReserved: 0,
        };

        Self { asbd }
    }

    pub fn sample_rate(&self) -> f64 {
        self.asbd.mSampleRate
    }

    pub fn sample_format(&self) -> SampleFormat {
        let flags = self.flags();

        let Some(format) = SampleFormat::from_flags_and_bits_per_sample(flags, self.asbd.mBitsPerChannel) else {
            // Should not happen if we went through TryFrom
            panic!("asbd is for an unsupported SampleFormat");
        };

        format
    }

    pub fn flags(&self) -> LinearPcmFlags {
        let Some(AudioFormat::LinearPCM(flags)) = AudioFormat::from_format_and_flag(self.asbd.mFormatID, Some(self.asbd.mFormatFlags)) else {
            // Should not happen if we went through TryFrom
            panic!("asbd is not LinearPcm");
        };

        flags
    }

    pub fn channels(&self) -> usize {
        self.asbd.mChannelsPerFrame as usize
    }
}

impl TryFrom<sys::AudioStreamBasicDescription> for StreamFormat {
    type Error = CAError;

    fn try_from(asbd: sys::AudioStreamBasicDescription) -> Result<Self, Self::Error> {
        // Require LinearPCM
        let Some(AudioFormat::LinearPCM(flags)) = AudioFormat::from_format_and_flag(asbd.mFormatID, Some(asbd.mFormatFlags)) else {
            return Err(AudioUnitError::FormatNotSupported.into());
        };

        if SampleFormat::from_flags_and_bits_per_sample(flags, asbd.mBitsPerChannel).is_none() {
            return Err(AudioUnitError::FormatNotSupported.into());
        }

        Ok(Self { asbd })
    }
}
