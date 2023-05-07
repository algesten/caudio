use bitflags::bitflags;
use std::fmt;
use sys;

bitflags! {
    pub struct ActionFlags: u32 {
        /// Called on a render notification Proc, which is called either before or after the
        /// render operation of the audio unit. If this flag is set, the proc is being called
        /// before the render operation is performed.
        ///
        /// **Available** in OS X v10.0 and later.
        const PRE_RENDER = sys::kAudioUnitRenderAction_PreRender;
        /// Called on a render notification Proc, which is called either before or after the
        /// render operation of the audio unit. If this flag is set, the proc is being called
        /// after the render operation is completed.
        ///
        /// **Available** in OS X v10.0 and later.
        const POST_RENDER = sys::kAudioUnitRenderAction_PostRender;
        /// This flag can be set in a render input callback (or in the audio unit's render
        /// operation itself) and is used to indicate that the render buffer contains only
        /// silence. It can then be used by the caller as a hint to whether the buffer needs to
        /// be processed or not.
        ///
        /// **Available** in OS X v10.2 and later.
        const OUTPUT_IS_SILENCE = sys::kAudioUnitRenderAction_OutputIsSilence;
        /// This is used with offline audio units (of type 'auol'). It is used when an offline
        /// unit is being preflighted, which is performed prior to when the actual offline
        /// rendering actions are performed. It is used for those cases where the offline
        /// process needs it (for example, with an offline unit that normalizes an audio file,
        /// it needs to see all of the audio data first before it can perform its
        /// normalization).
        ///
        /// **Available** in OS X v10.3 and later.
        const OFFLINE_PREFLIGHT = sys::kAudioOfflineUnitRenderAction_Preflight;
        /// Once an offline unit has been successfully preflighted, it is then put into its
        /// render mode. This flag is set to indicate to the audio unit that it is now in that
        /// state and that it should perform processing on the input data.
        ///
        /// **Available** in OS X v10.3 and later.
        const OFFLINE_RENDER = sys::kAudioOfflineUnitRenderAction_Render;
        /// This flag is set when an offline unit has completed either its preflight or
        /// performed render operation.
        ///
        /// **Available** in OS X v10.3 and later.
        const OFFLINE_COMPLETE = sys::kAudioOfflineUnitRenderAction_Complete;
        /// If this flag is set on the post-render call an error was returned by the audio
        /// unit's render operation. In this case, the error can be retrieved through the
        /// `lastRenderError` property and the audio data in `ioData` handed to the post-render
        /// notification will be invalid.
        ///
        /// **Available** in OS X v10.5 and later.
        const POST_RENDER_ERROR = sys::kAudioUnitRenderAction_PostRenderError;
        /// If this flag is set, then checks that are done on the arguments provided to render
        /// are not performed. This can be useful to use to save computation time in situations
        /// where you are sure you are providing the correct arguments and structures to the
        /// various render calls.
        ///
        /// **Available** in OS X v10.7 and later.
        const DO_NOT_CHECK_RENDER_ARGS = sys::kAudioUnitRenderAction_DoNotCheckRenderArgs;
    }
}

impl fmt::Display for ActionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            match self.bits() {
                sys::kAudioUnitRenderAction_PreRender => "PRE_RENDER",
                sys::kAudioUnitRenderAction_PostRender => "POST_RENDER",
                sys::kAudioUnitRenderAction_OutputIsSilence => "OUTPUT_IS_SILENCE",
                sys::kAudioOfflineUnitRenderAction_Preflight => "OFFLINE_PREFLIGHT",
                sys::kAudioOfflineUnitRenderAction_Render => "OFFLINE_RENDER",
                sys::kAudioOfflineUnitRenderAction_Complete => "OFFLINE_COMPLETE",
                sys::kAudioUnitRenderAction_PostRenderError => "POST_RENDER_ERROR",
                sys::kAudioUnitRenderAction_DoNotCheckRenderArgs => "DO_NOT_CHECK_RENDER_ARGS",
                _ => "<Unknown ActionFlags>",
            }
        )
    }
}
