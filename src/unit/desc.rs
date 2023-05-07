use std::ffi::CStr;
use std::mem;
use std::ops::Deref;
use std::ptr;

use core_foundation_sys::string::CFStringGetCString;

use crate::{try_os_status, CAError};

use super::Type;

/// Wrapper around an AudioComponentDescription.
#[derive(Debug, Clone)]
pub struct Description {
    name: String,
    version: Version,
    desc: sys::AudioComponentDescription,
}

impl PartialEq for Description {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

impl Eq for Description {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub bugfix: u8,
    pub stage: u8,
}

impl Description {
    pub fn first(ty: impl Into<Type>) -> Result<Description, CAError> {
        let ty = ty.into();
        let search = ty.into();

        let mut component = ptr::null_mut();
        component = unsafe { sys::AudioComponentFindNext(component, &search) };
        if component.is_null() {
            return Err(CAError::NoDescriptionFound(ty));
        }

        Ok(component.try_into()?)
    }

    pub fn list(ty: impl Into<Type>) -> Result<Vec<Description>, CAError> {
        let ty = ty.into();
        let search = ty.into();

        let mut ret = Vec::new();
        let mut component = ptr::null_mut();

        loop {
            component = unsafe { sys::AudioComponentFindNext(component, &search) };
            if component.is_null() {
                break;
            }

            ret.push(component.try_into()?);
        }

        Ok(ret)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> Version {
        self.version
    }
}

unsafe fn cfstring_ref_to_string(r: sys::CFStringRef) -> String {
    let len = sys::CFStringGetLength(r) + 1;
    let mut bytes = vec![0_i8; len as usize];

    CFStringGetCString(
        // sys::CFStringRef and core_foundation_sys should link to the same type.
        mem::transmute(r),
        bytes.as_mut_ptr(),
        len as isize,
        sys::kCFStringEncodingUTF8,
    );

    let c_str = CStr::from_ptr(bytes.as_ptr());
    c_str.to_str().unwrap().to_owned()
}

impl From<Type> for sys::AudioComponentDescription {
    fn from(value: Type) -> Self {
        sys::AudioComponentDescription {
            componentType: value.as_u32(),
            componentSubType: value.as_subtype_u32().unwrap_or(0),
            componentManufacturer: 0,
            componentFlags: 0,
            componentFlagsMask: 0,
        }
    }
}

impl TryFrom<sys::AudioComponent> for Description {
    type Error = CAError;

    fn try_from(component: sys::AudioComponent) -> Result<Self, Self::Error> {
        let name = unsafe {
            let mut name_ref: sys::CFStringRef = std::ptr::null();
            try_os_status!(sys::AudioComponentCopyName(component, &mut name_ref));
            cfstring_ref_to_string(name_ref)
        };

        let version = unsafe {
            let mut version = 0_u32;
            try_os_status!(sys::AudioComponentGetVersion(component, &mut version));
            let major = ((version >> 24) & 0xff) as u8;
            let minor = ((version >> 16) & 0xff) as u8;
            let bugfix = ((version >> 8) & 0xff) as u8;
            let stage = (version & 0xff) as u8;
            Version {
                major,
                minor,
                bugfix,
                stage,
            }
        };

        let desc = unsafe {
            let mut d = sys::AudioComponentDescription::default();
            try_os_status!(sys::AudioComponentGetDescription(component, &mut d));
            d
        };

        Ok(Description {
            name,
            version,
            desc,
        })
    }
}

impl Deref for Description {
    type Target = sys::AudioComponentDescription;

    fn deref(&self) -> &Self::Target {
        &self.desc
    }
}

impl TryFrom<&Description> for sys::AudioComponent {
    type Error = CAError;

    fn try_from(value: &Description) -> Result<Self, Self::Error> {
        let mut component = ptr::null_mut();
        component = unsafe { sys::AudioComponentFindNext(component, &**value) };
        if component.is_null() {
            return Err(CAError::NoComponentFound(value.clone()));
        }

        Ok(component)
    }
}
