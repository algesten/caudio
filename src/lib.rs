pub mod error;
pub use error::CAError;

pub mod format;

mod stream_format;
pub use stream_format::StreamFormat;

mod sample_format;
pub use sample_format::{Sample, SampleFormat};

mod audio_queue;
pub use audio_queue::{
    AudioQueueBuffer, AudioQueueInput, AudioQueueOutput, BorrowedAudioQueueBuffer,
};

macro_rules! try_os_status {
    ($expr:expr) => {
        CAError::from_os_status($expr)?
    };
}
pub(crate) use try_os_status;
