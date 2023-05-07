mod desc;
use std::marker::PhantomData;
use std::ptr;

pub use desc::{Description, Version};

pub mod types;
pub use types::Type;

use crate::format::Sample;
use crate::{try_os_status, CAError};

pub struct AudioUnit<S: Sample> {
    unit: sys::AudioUnit,
    initialized: bool,
    started: bool,
    _ph: PhantomData<S>,
}

impl<S: Sample> AudioUnit<S> {
    pub fn new(desc: Description) -> Result<Self, CAError> {
        let component: sys::AudioComponent = (&desc).try_into()?;

        let mut unit: sys::AudioUnit = ptr::null_mut();
        unsafe {
            try_os_status!(sys::AudioComponentInstanceNew(component, &mut unit,));
        }

        Ok(AudioUnit {
            unit,
            initialized: false,
            started: false,
            _ph: PhantomData,
        })
    }

    pub fn initialize(&mut self) -> Result<(), CAError> {
        if self.initialized {
            return Ok(());
        }
        unsafe {
            try_os_status!(sys::AudioUnitInitialize(self.unit));
        }
        self.initialized = true;
        Ok(())
    }

    pub fn uninitialize(&mut self) -> Result<(), CAError> {
        if !self.initialized {
            return Ok(());
        }
        if self.started {
            self.stop()?;
        }
        unsafe {
            try_os_status!(sys::AudioUnitUninitialize(self.unit));
        }
        self.initialized = false;
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), CAError> {
        if self.started {
            return Ok(());
        }
        if !self.initialized {
            self.initialize()?;
        }
        unsafe {
            try_os_status!(sys::AudioOutputUnitStart(self.unit));
        }
        self.started = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), CAError> {
        if !self.started {
            return Ok(());
        }
        unsafe {
            try_os_status!(sys::AudioOutputUnitStop(self.unit));
        }
        self.started = false;
        Ok(())
    }
}

impl<S: Sample> Drop for AudioUnit<S> {
    fn drop(&mut self) {
        self.stop().ok();
        self.uninitialize().ok();
        unsafe {
            sys::AudioComponentInstanceDispose(self.unit);
        }
    }
}
