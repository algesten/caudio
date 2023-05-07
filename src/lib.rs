pub mod error;
pub use error::CAError;

pub mod format;

pub mod queue;

macro_rules! try_os_status {
    ($expr:expr) => {
        CAError::from_os_status($expr)?
    };
}
pub(crate) use try_os_status;
