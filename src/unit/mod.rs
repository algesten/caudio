use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem;
use std::ptr;

mod desc;
pub use desc::{Description, Version};

pub mod types;
pub use types::Type;

mod buffer;
pub use buffer::AudioBuffers;

mod flags;
pub use flags::ActionFlags;

use crate::format::Sample;
use crate::{try_os_status, CAError};

pub struct AudioUnit<S: Sample> {
    unit: sys::AudioUnit,
    initialized: bool,
    started: bool,
    callback: Option<Box<RenderCallbackFnWrapper>>,
    _ph: PhantomData<S>,
}

/// The input and output **Scope**s.
///
/// More info [here](https://developer.apple.com/library/ios/documentation/AudioUnit/Reference/AudioUnitPropertiesReference/index.html#//apple_ref/doc/constant_group/Audio_Unit_Scopes)
/// and [here](https://developer.apple.com/library/mac/documentation/MusicAudio/Conceptual/AudioUnitProgrammingGuide/TheAudioUnit/TheAudioUnit.html).
#[derive(Copy, Clone, Debug)]
pub enum Scope {
    Global = 0,
    Input = 1,
    Output = 2,
    Group = 3,
    Part = 4,
    Note = 5,
    Layer = 6,
    LayerItem = 7,
}

/// Represents the **Input** and **Output** **Element**s.
///
/// These are used when specifying which **Element** we're setting the properties of.
#[derive(Copy, Clone, Debug)]
pub enum Element {
    Output = 0,
    Input = 1,
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
            callback: None,
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

    pub fn set_render_callback(
        &mut self,
        mut callback: impl RenderCallback<S> + 'static,
    ) -> Result<(), CAError> {
        assert!(self.callback.is_none(), "set render callback only once");

        // This closure gets around the problem of having a generic S..
        let input_proc_fn = move |io_action_flags: *mut sys::AudioUnitRenderActionFlags,
                                  in_time_stamp: *const sys::AudioTimeStamp,
                                  in_bus_number: sys::UInt32,
                                  in_number_frames: sys::UInt32,
                                  io_data: *mut sys::AudioBufferList|
              -> sys::OSStatus {
            let mut buffers = AudioBuffers::<'_, S>::borrow(io_data);
            unsafe {
                callback.render(
                    ActionFlags::from_bits_truncate(*io_action_flags),
                    *in_time_stamp,
                    in_bus_number,
                    in_number_frames as usize,
                    &mut buffers,
                );
            }
            0
        };

        let mut wrapper = Box::new(RenderCallbackFnWrapper {
            callback: Box::new(input_proc_fn),
        });

        let wrapper_ptr = &mut *wrapper as *mut RenderCallbackFnWrapper;
        self.callback = Some(wrapper);

        let render_callback = sys::AURenderCallbackStruct {
            inputProc: Some(input_proc),
            inputProcRefCon: wrapper_ptr as *mut c_void,
        };

        self.set_property(
            sys::kAudioUnitProperty_SetRenderCallback,
            Scope::Input,
            Element::Output,
            Some(&render_callback),
        )?;

        Ok(())
    }

    fn set_property<T>(
        &self,
        id: u32,
        scope: Scope,
        elem: Element,
        data: Option<&T>,
    ) -> Result<(), CAError> {
        let (data_ptr, size) = data
            .map(|data| {
                let ptr = data as *const _ as *const c_void;
                let size = mem::size_of::<T>() as u32;
                (ptr, size)
            })
            .unwrap_or_else(|| (::std::ptr::null(), 0));

        let scope = scope as u32;
        let elem = elem as u32;

        unsafe {
            try_os_status!(sys::AudioUnitSetProperty(
                self.unit, id, scope, elem, data_ptr, size
            ))
        }

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

pub trait RenderCallback<S: Sample> {
    fn render(
        &mut self,
        flags: ActionFlags,
        time: sys::AudioTimeStamp,
        bus: u32,
        frames: usize,
        buffers: &mut AudioBuffers<S>,
    );
}

impl<S: Sample, T: FnMut(ActionFlags, sys::AudioTimeStamp, u32, usize, &mut AudioBuffers<S>)>
    RenderCallback<S> for T
{
    fn render(
        &mut self,
        flags: ActionFlags,
        time: sys::AudioTimeStamp,
        bus: u32,
        frames: usize,
        buffers: &mut AudioBuffers<S>,
    ) {
        (self)(flags, time, bus, frames, buffers)
    }
}

type RenderCallbackFn = dyn FnMut(
    *mut sys::AudioUnitRenderActionFlags,
    *const sys::AudioTimeStamp,
    sys::UInt32,
    sys::UInt32,
    *mut sys::AudioBufferList,
) -> sys::OSStatus;

struct RenderCallbackFnWrapper {
    callback: Box<RenderCallbackFn>,
}

/// Callback procedure that will be called each time our audio_unit requests audio.
extern "C" fn input_proc(
    in_ref_con: *mut c_void,
    io_action_flags: *mut sys::AudioUnitRenderActionFlags,
    in_time_stamp: *const sys::AudioTimeStamp,
    in_bus_number: sys::UInt32,
    in_number_frames: sys::UInt32,
    io_data: *mut sys::AudioBufferList,
) -> sys::OSStatus {
    let wrapper = in_ref_con as *mut RenderCallbackFnWrapper;
    unsafe {
        (*(*wrapper).callback)(
            io_action_flags,
            in_time_stamp,
            in_bus_number,
            in_number_frames,
            io_data,
        )
    }
}

#[cfg(test)]
mod test {
    use super::types::EffectType;
    use super::*;

    #[test]
    fn instantiate_reverb() {
        let d = Description::first(EffectType::MatrixReverb).unwrap();
        let mut u = AudioUnit::<f32>::new(d).unwrap();
        u.initialize().unwrap();
    }
}
