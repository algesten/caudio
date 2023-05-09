use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};

use crate::format::Sample;

// This is how we want the layout of the AudioBufferList with the DST AudioBuffer
// and pointers to another memory segment with the actual audio data.
//
//     AudioBufferList                     Audio data
//    ┌──────────────┐                 ┌▶┌──────────────┐
//    │mNumberBuffers│               ┌─┘ │              │
//    ├──────────────┤             ┌─┘   │              │
//    ├ ─ ─ ─ ─ ─ ─ ─│           ┌─┘     │              │
//    │   padding    │         ┌─┘       │              │
//    ├ ─ ─ ─ ─ ─ ─ ─│       ┌─┘         │              │
//    ╠══════════════╣     ┌─┘         ┌▶├ ─ ─ ─ ─ ─ ─ ─│
//    ║  data_size   ║   ┌─┘         ┌─┘ │              │
//    ╠ ─ ─ ─ ─ ─ ─ ─║ ┌─┘         ┌─┘   │              │
//    ║     data     ║─┘         ┌─┘     │              │
//    ╠══════════════╣         ┌─┘       │     data     │
//    ╠══════════════╣       ┌─┘         │              │
//    ║  data_size   ║     ┌─┘           │              │
//    ╠ ─ ─ ─ ─ ─ ─ ─║  ┌──┘           ┌▶├ ─ ─ ─ ─ ─ ─ ─│
//    ║     data     ║──┘           ┌──┘ │              │
//    ╠══════════════╣           ┌──┘    │              │
//    ╠══════════════╣        ┌──┘       │              │
//    ║  data_size   ║     ┌──┘          │              │
//    ╠ ─ ─ ─ ─ ─ ─ ─║  ┌──┘             │              │
//    ║     data     ║──┘                │              │
//    ╚══════════════╝                   └──────────────┘
//
//

/// Wrapper around AudioBufferList.
pub struct AudioBufferList<S: Sample> {
    // When we create an owned list, the actual struct is in the _audio_buffer_list field
    // below. This is because the C-struct looks like this:
    // struct AudioBufferList
    // {
    //     UInt32      mNumberBuffers;
    //     AudioBuffer mBuffers[1]; // this is a variable length array of mNumberBuffers elements
    // }
    // I.e. we have a dynamically growing array as last field.
    //
    // When we have a borrowed list, we don't use the _audio_buffer_list at all.
    list: *mut sys::AudioBufferList,

    // Backing buffer for the list pointer when we have owned data.
    _audio_buffer_list: Box<[u8]>,

    // Backing buffer for all the audio buffers.
    _audio_data: Box<[S]>,
}

/// Overlay type over the actual sys::AudioBuffer type.
#[repr(C)]
pub struct AudioBuffer<S: Sample> {
    channels: u32,
    data_byte_size: u32,
    data: *mut c_void,
    _ph: PhantomData<S>, // zero sized
}

impl<S: Sample> AudioBufferList<S> {
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
        // Need at least one buffer to be valid.
        assert!(buffers >= 1);

        // Allocate space for the sys::AudioBufferList and all additional array
        // elements we have after it. The struct has space for 1 audio buffer.
        let list_byte_size = mem::size_of::<sys::AudioBufferList>();
        let buffer_byte_size = mem::size_of::<sys::AudioBuffer>();

        // -1 because there is space for one buffer in the struct.
        let buffer_array_size = (buffers - 1) * buffer_byte_size;

        let mut audio_buffer_list =
            vec![0_u8; list_byte_size + buffer_array_size].into_boxed_slice();

        let list = &mut *audio_buffer_list as *mut _ as *mut sys::AudioBufferList;
        let to_fill = unsafe {
            (*list).mNumberBuffers = buffers as u32;
            let ptr = &mut (*list).mBuffers as *mut _ as *mut AudioBuffer<S>;
            std::slice::from_raw_parts_mut(ptr, buffers)
        };

        let samples_per_buffer = channels * frames;
        let samples_total = buffers * samples_per_buffer;
        let bytes_per_buffer = samples_per_buffer * mem::size_of::<S>();

        // Allocate all data we need in one chunk, we take pointers into it.
        let mut audio_data = vec![S::default(); samples_total].into_boxed_slice();

        {
            let mut left = &mut audio_data[..];
            for buffer in to_fill.iter_mut() {
                // Chunk off the amount we need for this buffer.
                let (data, _left) = left.split_at_mut(samples_per_buffer);

                // Keep track of how much we have left.
                left = _left;

                buffer.channels = channels as u32;
                buffer.data_byte_size = bytes_per_buffer as u32;
                buffer.data = data as *mut [S] as *mut c_void;
            }
        }

        Self {
            list,
            _audio_buffer_list: audio_buffer_list,
            _audio_data: audio_data,
        }
    }

    // Use a borrowed buffer as provided by core audio in render callbacks etc.
    pub(crate) fn borrow(list: *mut sys::AudioBufferList) -> Self {
        Self {
            list,
            // Dummy values since list is borrowed from _some other place_ that manages
            // the deallocation.
            _audio_buffer_list: vec![].into_boxed_slice(),
            _audio_data: vec![].into_boxed_slice(),
        }
    }

    pub(crate) fn as_sys_list(&mut self) -> *mut sys::AudioBufferList {
        self.list
    }

    /// Slice of contained buffers.
    ///
    /// Same as using the Deref trait.
    pub fn buffers(&self) -> &[AudioBuffer<S>] {
        unsafe {
            let len = (*self.list).mNumberBuffers as usize;
            let ptr = &(*self.list).mBuffers as *const _ as *const AudioBuffer<S>;
            std::slice::from_raw_parts(ptr, len)
        }
    }

    /// Slice of mutable contained buffers.
    ///
    /// Same as using the DerefMut trait.
    pub fn buffers_mut(&mut self) -> &mut [AudioBuffer<S>] {
        unsafe {
            let len = (*self.list).mNumberBuffers as usize;
            let ptr = &mut (*self.list).mBuffers as *mut _ as *mut AudioBuffer<S>;
            std::slice::from_raw_parts_mut(ptr, len)
        }
    }
}

impl<S: Sample> AudioBuffer<S> {
    /// Number of channels.
    pub fn channels(&self) -> usize {
        self.channels as usize
    }

    /// Number of frames.
    ///
    /// Channels are assumed to be interleaved, which means 2 channels are
    /// organized as [L,R,L,R,...]. The number of frames is therefore the
    /// total length of the buffer [`AudioBuffer::len()`] divided by the channels.
    pub fn frames(&self) -> usize {
        self.len() / self.channels()
    }

    /// Samples as a slice.
    pub fn samples(&self) -> &[S] {
        unsafe {
            let AudioBuffer {
                data_byte_size,
                data,
                ..
            } = self;

            let len = *data_byte_size as usize / mem::size_of::<S>();

            std::slice::from_raw_parts(*data as *mut S, len)
        }
    }

    /// Samples as a mutable slice.
    pub fn samples_mut(&mut self) -> &mut [S] {
        unsafe {
            let AudioBuffer {
                data_byte_size,
                data,
                ..
            } = self;

            let len = *data_byte_size as usize / mem::size_of::<S>();

            std::slice::from_raw_parts_mut(*data as *mut S, len)
        }
    }
}

impl<S: Sample> Deref for AudioBufferList<S> {
    type Target = [AudioBuffer<S>];

    fn deref(&self) -> &Self::Target {
        self.buffers()
    }
}

impl<S: Sample> DerefMut for AudioBufferList<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffers_mut()
    }
}

impl<S: Sample> Deref for AudioBuffer<S> {
    type Target = [S];

    fn deref(&self) -> &Self::Target {
        self.samples()
    }
}

impl<S: Sample> DerefMut for AudioBuffer<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.samples_mut()
    }
}

impl<S: Sample> fmt::Debug for AudioBufferList<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buffers: &[AudioBuffer<S>] = &self;
        f.debug_struct("AudioBuffers")
            .field("buffers", &buffers)
            .finish()
    }
}

impl<S: Sample> fmt::Debug for AudioBuffer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let samples: &[S] = &self;
        f.debug_struct("AudioBuffer")
            .field("samples", &samples)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn owned_non_interleaved() {
        let b = AudioBufferList::<f32>::new(2, 1, 512);
        assert_eq!(b.len(), 2);
        assert_eq!(b.buffers()[0].channels, 1);
        assert_eq!(b.buffers()[1].frames(), 512);
    }

    #[test]
    fn owned_interleaved() {
        let b = AudioBufferList::<f32>::new(1, 2, 512);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].channels(), 2);
        assert_eq!(b[0].frames(), 512);
    }

    #[test]
    fn debug_print() {
        let b = AudioBufferList::<f32>::new(1, 2, 512);
        println!("{:?}", b);
    }
}
