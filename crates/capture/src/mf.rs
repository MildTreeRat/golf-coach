//! Media Foundation: the Windows backend. [M21 P1]
//!
//! Windows first because that is the machine this repo is built on (ADR-031 §4). The module is
//! behind `cfg(windows)` and the types it returns are [`crate::device`]'s, so V4L2 and
//! AVFoundation slot in beside it rather than under it.
//!
//! # Two calls, and the second one opens the camera
//!
//! `MFEnumDeviceSources` is cheap and answers "what is plugged in": a friendly name and a
//! symbolic link per device, no hardware touched. Capabilities are not in that answer —
//! `IMFMediaTypeHandler` is reached through `ActivateObject`, which **opens the device**. So a
//! camera already in use by another application is expected to enumerate and then refuse to
//! describe itself, and that is a routine condition rather than an error: it becomes
//! [`Capabilities::Unavailable`] carrying the platform's own `HRESULT` message, and the device
//! still appears in the list. A caller must not read it as a camera with no formats, which is
//! why that enum has two variants.
//!
//! # Everything past the empty list is unexercised, and a reader should know it
//!
//! M21 P1 had no camera to attach (see the crate docs), so the only path this module has ever
//! *run* is `MFStartup` → enumerate → zero devices → `MFShutdown`. `read`, `capabilities`,
//! `describe` and `media_types` are written against Media Foundation's documented behaviour and
//! have executed against nothing. Their unit tests cover the pure functions they lean on —
//! [`pixel_format`] and [`unpack`] — and not the COM calls around them. The first `list` with a
//! camera plugged in is the real test, and it is what finishes the phase.

use std::ffi::c_void;

use windows::core::{GUID, PWSTR};
use windows::Win32::Media::MediaFoundation::{
    IMFActivate, IMFAttributes, IMFMediaSource, IMFMediaType, IMFMediaTypeHandler,
    IMFPresentationDescriptor, IMFStreamDescriptor, MFCreateAttributes, MFEnumDeviceSources,
    MFShutdown, MFStartup, MFSTARTUP_NOSOCKET, MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
    MF_MT_SUBTYPE, MF_VERSION,
};
use windows::Win32::System::Com::CoTaskMemFree;

use crate::device::{Capabilities, Capability, Device, DeviceId, PixelFormat};
use crate::Error;

/// Every video capture device Media Foundation can see, with its formats where it would say.
pub(crate) fn enumerate() -> Result<Vec<Device>, Error> {
    let _mf = Platform::start()?;
    // SAFETY: every pointer below is one MF just wrote or one we allocated, and the two
    // allocations MF hands back (the activate array, and each string) are freed on every path.
    unsafe {
        let attributes = vidcap_filter()?;
        let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
        let mut count: u32 = 0;
        MFEnumDeviceSources(&attributes, &mut activates, &mut count)
            .map_err(|e| Error::Platform(format!("MFEnumDeviceSources failed: {e}")))?;

        let mut devices = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            // `take` moves the interface out of the array, so it is dropped — and released —
            // when it goes out of scope. Leaving it in place and freeing the array would leak
            // one reference per camera, every time a session checks its device is still there.
            if let Some(activate) = (*activates.add(i)).take() {
                devices.push(read(&activate));
            }
        }
        CoTaskMemFree(Some(activates as *const c_void));
        Ok(devices)
    }
}

/// `MFStartup`/`MFShutdown` as a scope.
///
/// A guard rather than a pair of calls because `enumerate` has several failure paths and every
/// one of them has to shut the platform down: a process that leaves Media Foundation started is
/// holding the capture stack up for whatever runs next, and a session is expected to call this
/// repeatedly.
struct Platform;

impl Platform {
    fn start() -> Result<Self, Error> {
        // No `CoInitializeEx` beside this, deliberately: `MFStartup` initialises COM itself in
        // the apartment it wants, and adding an explicit one would make the apartment model this
        // function's decision on behalf of every caller. P2's capture loop runs on a thread of
        // its own and gets to answer that for itself.
        //
        // SAFETY: matched by the `MFShutdown` in `Drop`, and no MF call is made outside it.
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) }
            .map_err(|e| Error::Platform(format!("MFStartup failed: {e}")))?;
        Ok(Self)
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        // SAFETY: paired with the `MFStartup` in `start`, which is the only way to get here.
        let _ = unsafe { MFShutdown() };
    }
}

/// The attribute store that narrows `MFEnumDeviceSources` to video capture devices.
///
/// # Safety
/// Must be called with Media Foundation started.
unsafe fn vidcap_filter() -> Result<IMFAttributes, Error> {
    let mut attributes: Option<IMFAttributes> = None;
    MFCreateAttributes(&mut attributes, 1)
        .map_err(|e| Error::Platform(format!("MFCreateAttributes failed: {e}")))?;
    let attributes = attributes
        .ok_or_else(|| Error::Platform("MFCreateAttributes returned no attribute store".into()))?;
    attributes
        .SetGUID(
            &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
            &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
        )
        .map_err(|e| Error::Platform(format!("could not ask for video capture devices: {e}")))?;
    Ok(attributes)
}

/// One enumerated device, read into [`Device`].
///
/// # Safety
/// `activate` must be a live `IMFActivate` from `MFEnumDeviceSources`.
unsafe fn read(activate: &IMFActivate) -> Device {
    // A device with no symbolic link should not exist; if one does, it gets its friendly name as
    // an identity rather than being dropped from the list. A camera the host can see and this
    // crate does not report is the worse failure — a session would enumerate past it silently.
    let id = allocated_string(
        activate,
        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
    );
    let name = allocated_string(activate, &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME)
        .unwrap_or_else(|| "(unnamed)".to_string());
    let id = DeviceId::new(id.unwrap_or_else(|| name.clone()));
    Device {
        id,
        name,
        capabilities: capabilities(activate),
    }
}

/// Read one `PWSTR`-valued attribute, freeing what MF allocated for it.
///
/// # Safety
/// `activate` must be a live `IMFActivate`.
unsafe fn allocated_string(activate: &IMFActivate, key: &GUID) -> Option<String> {
    let mut value = PWSTR::null();
    let mut len: u32 = 0;
    activate
        .GetAllocatedString(key, &mut value, &mut len)
        .ok()?;
    if value.is_null() {
        return None;
    }
    let owned = value.to_string().ok();
    CoTaskMemFree(Some(value.as_ptr() as *const c_void));
    owned
}

/// What the device says it can be asked for — or why it would not say.
///
/// # Safety
/// `activate` must be a live `IMFActivate`.
unsafe fn capabilities(activate: &IMFActivate) -> Capabilities {
    let source: IMFMediaSource = match activate.ActivateObject() {
        Ok(s) => s,
        // The case this is written for: the camera is open in another application. Reported as
        // the platform phrased it, because "could not open" and "access denied" send a user to
        // different places.
        Err(e) => return Capabilities::Unavailable(format!("could not open the device: {e}")),
    };
    let listed = describe(&source);
    // Shut the source down before the activate: `ShutdownObject` releases MF's cached object,
    // and a source left running holds the camera open against the next `list`.
    let _ = source.Shutdown();
    let _ = activate.ShutdownObject();

    match listed {
        Ok(mut formats) => {
            formats.sort();
            formats.dedup();
            Capabilities::Reported(formats)
        }
        Err(detail) => Capabilities::Unavailable(detail),
    }
}

/// Walk the presentation descriptor's media types.
///
/// # Safety
/// `source` must be a live `IMFMediaSource`.
unsafe fn describe(source: &IMFMediaSource) -> Result<Vec<Capability>, String> {
    let descriptor: IMFPresentationDescriptor = source
        .CreatePresentationDescriptor()
        .map_err(|e| format!("the device would not describe itself: {e}"))?;
    let streams = descriptor
        .GetStreamDescriptorCount()
        .map_err(|e| format!("the device reported no stream count: {e}"))?;

    let mut formats = Vec::new();
    for i in 0..streams {
        let mut selected = windows::core::BOOL::default();
        let mut stream: Option<IMFStreamDescriptor> = None;
        if descriptor
            .GetStreamDescriptorByIndex(i, &mut selected, &mut stream)
            .is_err()
        {
            continue;
        }
        let Some(stream) = stream else { continue };
        let Ok(handler) = stream.GetMediaTypeHandler() else {
            continue;
        };
        formats.extend(media_types(&handler));
    }
    Ok(formats)
}

/// Every media type one stream handler offers.
///
/// Individual types are skipped rather than failing the device: a driver that advertises one
/// malformed format still offers the others, and a camera reported as undescribable because of
/// its fourteenth format would be a wrong answer about working hardware.
///
/// # Safety
/// `handler` must be a live `IMFMediaTypeHandler`.
unsafe fn media_types(handler: &IMFMediaTypeHandler) -> Vec<Capability> {
    let Ok(count) = handler.GetMediaTypeCount() else {
        return Vec::new();
    };
    let mut formats = Vec::with_capacity(count as usize);
    for j in 0..count {
        let Ok(media_type) = handler.GetMediaTypeByIndex(j) else {
            continue;
        };
        if let Some(capability) = capability_of(&media_type) {
            formats.push(capability);
        }
    }
    formats
}

/// One `IMFMediaType` as a [`Capability`], or `None` if it is not a described video format.
///
/// # Safety
/// `media_type` must be a live `IMFMediaType`.
unsafe fn capability_of(media_type: &IMFMediaType) -> Option<Capability> {
    // Both are `Packed2UINT32asUINT64`: high word first. Frame size is (width, height) and
    // frame rate is (numerator, denominator) — the rate is kept as the ratio it arrives as,
    // because 30000/1001 is a real UVC rate and it is not 30.
    let (width, height) = unpack(media_type.GetUINT64(&MF_MT_FRAME_SIZE).ok()?);
    let (fps_numerator, fps_denominator) = unpack(media_type.GetUINT64(&MF_MT_FRAME_RATE).ok()?);
    let subtype = media_type.GetGUID(&MF_MT_SUBTYPE).ok()?;
    Some(Capability {
        width,
        height,
        fps_numerator,
        fps_denominator,
        format: pixel_format(&subtype),
    })
}

/// Split one of MF's packed pairs into its high and low halves.
fn unpack(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, (packed & 0xFFFF_FFFF) as u32)
}

/// A media subtype GUID as a pixel format.
///
/// Every UVC format's subtype is a FourCC wearing a GUID: `{FOURCC-0000-0010-8000-00AA00389B71}`,
/// with the FourCC's four characters in `Data1`'s little-endian bytes. Anything that does not
/// match that template is kept as its GUID rather than being forced into four characters — an
/// unrecognised format is still a format the camera offers.
fn pixel_format(subtype: &GUID) -> PixelFormat {
    const MF_FOURCC_TAIL: [u8; 8] = [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71];
    if subtype.data2 == 0x0000 && subtype.data3 == 0x0010 && subtype.data4 == MF_FOURCC_TAIL {
        PixelFormat::FourCc(subtype.data1.to_le_bytes())
    } else {
        PixelFormat::Other(format!("{subtype:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOURCC_TAIL: [u8; 8] = [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71];

    #[test]
    fn a_fourcc_subtype_reads_as_its_four_characters() {
        // MFVideoFormat_MJPG, MFVideoFormat_NV12 and MFVideoFormat_YUY2 as Windows defines them.
        let mjpg = GUID::from_values(0x4750_4A4D, 0x0000, 0x0010, FOURCC_TAIL);
        let nv12 = GUID::from_values(0x3231_564E, 0x0000, 0x0010, FOURCC_TAIL);
        let yuy2 = GUID::from_values(0x3259_5559, 0x0000, 0x0010, FOURCC_TAIL);
        assert_eq!(pixel_format(&mjpg), PixelFormat::FourCc(*b"MJPG"));
        assert_eq!(pixel_format(&nv12), PixelFormat::FourCc(*b"NV12"));
        assert_eq!(pixel_format(&yuy2), PixelFormat::FourCc(*b"YUY2"));
    }

    #[test]
    fn a_subtype_outside_the_template_is_kept_rather_than_forced() {
        // MFVideoFormat_H264_ES: a real MF subtype with a different tail, so the four bytes of
        // `data1` are not a FourCC and must not be printed as one.
        let other = GUID::from_values(
            0x3F40_F4F0,
            0x5622,
            0x4FF8,
            [0xB6, 0xD8, 0xA1, 0x7A, 0x58, 0x4B, 0xEE, 0x5E],
        );
        match pixel_format(&other) {
            PixelFormat::Other(s) => assert!(!s.is_empty()),
            found => panic!("a non-FourCC subtype read as {found:?}"),
        }
    }

    #[test]
    fn packed_pairs_split_high_word_first() {
        // 1920x1080 and 30000/1001 as MF packs them.
        assert_eq!(unpack((1920u64 << 32) | 1080), (1920, 1080));
        assert_eq!(unpack((30_000u64 << 32) | 1001), (30_000, 1001));
    }
}
