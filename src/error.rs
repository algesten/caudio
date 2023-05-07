use sys::OSStatus;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum CAError {
    #[error("audio error: {0}")]
    AudioError(#[from] AudioError),
    #[error("audio codec error: {0}")]
    AudioCodecError(#[from] AudioCodecError),
    #[error("audio format error: {0}")]
    AudioFormatError(#[from] AudioFormatError),
    #[error("audio unit error: {0}")]
    AudioUnitError(#[from] AudioUnitError),
    #[error("other: {0}")]
    Other(String),
}

impl<S: Into<String>> From<S> for CAError {
    fn from(value: S) -> Self {
        CAError::Other(value.into())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum AudioError {
    #[error("unimplemented")]
    Unimplemented = -4,
    #[error("file not found")]
    FileNotFound = -43,
    #[error("file permission problem")]
    FilePermission = -54,
    #[error("too many open files")]
    TooManyFilesOpen = -42,
    #[error("bad file path")]
    BadFilePath = 561017960,
    #[error("parameter error")]
    Param = -50,
    #[error("memory full")]
    MemFull = -108,
}

impl AudioError {
    pub fn from_os_status(status: OSStatus) -> Option<Self> {
        use AudioError::*;
        match status {
            -4 => Some(Unimplemented),
            -43 => Some(FileNotFound),
            -54 => Some(FilePermission),
            -42 => Some(TooManyFilesOpen),
            561017960 => Some(BadFilePath),
            -50 => Some(Param),
            -108 => Some(MemFull),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum AudioCodecError {
    #[error("unspecified")]
    Unspecified = 2003329396,
    #[error("unknown property")]
    UnknownProperty = 2003332927,
    #[error("bad property size")]
    BadPropertySize = 561211770,
    #[error("illegal operation")]
    IllegalOperation = 1852797029,
    #[error("unsupported format")]
    UnsupportedFormat = 560226676,
    #[error("state error")]
    State = 561214580,
    #[error("not enough buffer space")]
    NotEnoughBufferSpace = 560100710,
}

impl AudioCodecError {
    pub fn from_os_status(status: OSStatus) -> Option<Self> {
        use AudioCodecError::*;
        match status {
            2003329396 => Some(Unspecified),
            2003332927 => Some(UnknownProperty),
            561211770 => Some(BadPropertySize),
            1852797029 => Some(IllegalOperation),
            560226676 => Some(UnsupportedFormat),
            561214580 => Some(State),
            560100710 => Some(NotEnoughBufferSpace),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum AudioFormatError {
    #[error("unspecified")]
    Unspecified, // 'what'
    #[error("unsupported property")]
    UnsupportedProperty, // 'prop'
    #[error("bad property size")]
    BadPropertySize, // '!siz'
    #[error("bad specifier size")]
    BadSpecifierSize, // '!spc'
    #[error("unsupported data format")]
    UnsupportedDataFormat = 1718449215, // 'fmt?'
    #[error("unknown format")]
    UnknownFormat, // '!fmt'
}

impl AudioFormatError {
    pub fn from_os_status(status: OSStatus) -> Option<Self> {
        use AudioFormatError::*;
        match status {
            1718449215 => Some(UnsupportedDataFormat),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum AudioUnitError {
    #[error("invalid property")]
    InvalidProperty = -10879,
    #[error("invalid parameter")]
    InvalidParameter = -10878,
    #[error("invalid element")]
    InvalidElement = -10877,
    #[error("no connection")]
    NoConnection = -10876,
    #[error("failed initialization")]
    FailedInitialization = -10875,
    #[error("too many frames to process")]
    TooManyFramesToProcess = -10874,
    #[error("invalid file")]
    InvalidFile = -10871,
    #[error("format not supported")]
    FormatNotSupported = -10868,
    #[error("uninitialized")]
    Uninitialized = -10867,
    #[error("invalid scope")]
    InvalidScope = -10866,
    #[error("property not writable")]
    PropertyNotWritable = -10865,
    #[error("cannot do in current context")]
    CannotDoInCurrentContext = -10863,
    #[error("invalid property value")]
    InvalidPropertyValue = -10851,
    #[error("property not in use")]
    PropertyNotInUse = -10850,
    #[error("initialized")]
    Initialized = -10849,
    #[error("invalid offline render")]
    InvalidOfflineRender = -10848,
    #[error("unauthorized")]
    Unauthorized = -10847,
}

impl AudioUnitError {
    pub fn from_os_status(status: OSStatus) -> Option<Self> {
        use AudioUnitError::*;
        match status {
            -10879 => Some(InvalidProperty),
            -10878 => Some(InvalidParameter),
            -10877 => Some(InvalidElement),
            -10876 => Some(NoConnection),
            -10875 => Some(FailedInitialization),
            -10874 => Some(TooManyFramesToProcess),
            -10871 => Some(InvalidFile),
            -10868 => Some(FormatNotSupported),
            -10867 => Some(Uninitialized),
            -10866 => Some(InvalidScope),
            -10865 => Some(PropertyNotWritable),
            -10863 => Some(CannotDoInCurrentContext),
            -10851 => Some(InvalidPropertyValue),
            -10850 => Some(PropertyNotInUse),
            -10849 => Some(Initialized),
            -10848 => Some(InvalidOfflineRender),
            -10847 => Some(Unauthorized),
            _ => None,
        }
    }
}
