#[cfg(target_os = "windows")]
mod dcomp;
mod ffi;
mod runtime;

#[cfg(target_os = "windows")]
#[allow(unused_imports)]
pub use dcomp::SpineDcompRenderer;
#[allow(unused_imports)]
pub use runtime::{
    BoneWorldTransform, SpineAnimationInfo, SpineAttachmentType, SpineBlendMode, SpineBoneInfo,
    SpineRenderBatch, SpineRenderFrame, SpineRuntime, SpineSlotInfo, SpineTimelineInfo,
    SpineTimelineTargetKind, SpineTimelineType, SpineVertex,
};
