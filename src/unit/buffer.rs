use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};

use crate::format::Sample;

/// Wrapper around AudioBufferList.
pub struct AudioBuffers<S: Sample> {
    // For owned buffers, this contains pointers into buffers and all_data. As long as those
    // boxed slices are alive, the SysAudioBufferList should be valid.
    // For borrowed data, list contains pointers into _some other place_, and
    // is only valid for the lifetime 'a.
    list: Box<SysAudioBufferList>,
    free_on_drop: bool,
    _buffers: Box<[sys::AudioBuffer]>,
    _all_data: Box<[S]>,
}

impl<S: Sample> AudioBuffers<S> {
    /// Creates a new owned audio buffer.
    ///
    /// * `buffers` is how many buffers we want. For non-interleaved stereo data, we
    /// need 2 buffers. For interleaved stereo data we need 1 buffer.
    ///
    /// * `channels` is how many interleaved channels we have _per buffer_. For
    /// interleaved stereo, this value is 2. For non-interleaved 1.
    ///
    /// * `frames` is how many frames we have per buffer. The number of samples
    /// that can go in each buffer is `channels` * `frames`.
    pub fn new(buffers: usize, channels: usize, frames: usize) -> Self {
        let samples_per_buffer = channels * frames;
        let bytes_per_buffer = samples_per_buffer * mem::size_of::<S>();
        let samples_total = buffers * samples_per_buffer;

        // Allocate all data we need in one chunk, we take pointer into it.
        let mut all_data = vec![S::default(); samples_total].into_boxed_slice();

        let mut bufs = Vec::with_capacity(buffers);

        {
            let mut left = &mut all_data[..];
            for _ in 0..buffers {
                // Chunk off the amount we need for this buffer.
                let (data, _left) = left.split_at_mut(samples_per_buffer);

                // Keep track of how much we have left.
                left = _left;

                let buf = sys::AudioBuffer {
                    mNumberChannels: channels as u32,
                    mDataByteSize: bytes_per_buffer as u32,
                    mData: data as *mut [S] as *mut c_void,
                };
                bufs.push(buf);
            }
        }

        let mut bufs = bufs.into_boxed_slice();

        let list = Box::new(SysAudioBufferList {
            mNumberBuffers: bufs.len() as u32,
            mBuffers: &mut *bufs as *mut _ as *mut sys::AudioBuffer,
        });

        Self {
            list,
            free_on_drop: true,
            _buffers: bufs,
            _all_data: all_data,
        }
    }
    pub(crate) fn borrow(list: *mut sys::AudioBufferList) -> Self {
        let list = unsafe {
            SysAudioBufferList {
                mNumberBuffers: (*list).mNumberBuffers,
                mBuffers: &mut (*list).mBuffers as *mut sys::AudioBuffer,
            }
        };

        Self {
            list: Box::new(list),
            free_on_drop: false,
            // Dummy values since list is borrowed from _some other place_ that manages
            // the deallocation.
            _buffers: vec![].into_boxed_slice(),
            _all_data: vec![].into_boxed_slice(),
        }
    }

    pub(crate) fn as_sys_list(&mut self) -> *mut sys::AudioBufferList {
        &mut *self.list as *mut SysAudioBufferList as *mut sys::AudioBufferList
    }

    pub fn buffers(&self) -> usize {
        self.len()
    }

    pub fn channels(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            let first = &self[0];
            first.mNumberChannels as usize
        }
    }

    pub fn frames(&self) -> usize {
        let channels = self.channels();
        if channels == 0 {
            0
        } else {
            self[0].len() / channels
        }
    }
}

// For some reason coreaudio-sys has a 1 field array for mBuffers and we want
// to be more generic.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_snake_case)]
pub(crate) struct SysAudioBufferList {
    mNumberBuffers: u32,
    mBuffers: *mut sys::AudioBuffer,
}

/// Overlay type over the actual sys::AudioBuffer type.
///
/// This helps us to implement Deref.
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_snake_case)]
pub struct DerefAudioBuffer<S: Sample> {
    mNumberChannels: u32,
    mDataByteSize: u32,
    mData: *mut c_void,
    _ph: PhantomData<S>,
}

impl<S: Sample> Deref for AudioBuffers<S> {
    type Target = [DerefAudioBuffer<S>];

    fn deref(&self) -> &Self::Target {
        unsafe {
            let SysAudioBufferList {
                mNumberBuffers,
                mBuffers,
            } = *self.list;

            std::slice::from_raw_parts(
                mBuffers as *mut DerefAudioBuffer<S>,
                mNumberBuffers as usize,
            )
        }
    }
}

impl<S: Sample> DerefMut for AudioBuffers<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let SysAudioBufferList {
                mNumberBuffers,
                mBuffers,
            } = *self.list;

            std::slice::from_raw_parts_mut(
                mBuffers as *mut DerefAudioBuffer<S>,
                mNumberBuffers as usize,
            )
        }
    }
}

impl<S: Sample> Deref for DerefAudioBuffer<S> {
    type Target = [S];

    fn deref(&self) -> &Self::Target {
        unsafe {
            let DerefAudioBuffer {
                mDataByteSize,
                mData,
                ..
            } = self;

            let len = *mDataByteSize as usize / mem::size_of::<S>();

            std::slice::from_raw_parts(*mData as *mut S, len)
        }
    }
}

impl<S: Sample> DerefMut for DerefAudioBuffer<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let DerefAudioBuffer {
                mDataByteSize,
                mData,
                ..
            } = self;

            let len = *mDataByteSize as usize / mem::size_of::<S>();

            std::slice::from_raw_parts_mut(*mData as *mut S, len)
        }
    }
}

impl<S: Sample> fmt::Debug for AudioBuffers<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buffers: &[DerefAudioBuffer<S>] = &self;
        f.debug_struct("AudioBuffers")
            .field("buffers", &buffers)
            .finish()
    }
}

impl<S: Sample> fmt::Debug for DerefAudioBuffer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let samples: &[S] = &self;
        f.debug_struct("Buffer").field("samples", &samples).finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn owned_empty() {
        let b = AudioBuffers::<f32>::new(0, 2, 5);
        assert_eq!(b.len(), 0);
        assert_eq!(b.buffers(), 0);
        assert_eq!(b.channels(), 0);
        assert_eq!(b.frames(), 0);
    }

    #[test]
    fn owned_non_interleaved() {
        let b = AudioBuffers::<f32>::new(2, 1, 512);
        assert_eq!(b.len(), 2);
        assert_eq!(b.buffers(), 2);
        assert_eq!(b.channels(), 1);
        assert_eq!(b.frames(), 512);
    }

    #[test]
    fn owned_interleaved() {
        let b = AudioBuffers::<f32>::new(1, 2, 512);
        assert_eq!(b.buffers(), 1);
        assert_eq!(b.channels(), 2);
        assert_eq!(b.frames(), 512);
    }

    #[test]
    fn debug_print() {
        let b = AudioBuffers::<f32>::new(1, 2, 512);
        println!("{:?}", &*b);
    }
}
