use std::time::Instant;

use slint::{ComponentHandle, LogicalSize};

use crate::{
    PetWindow, behavior::Behavior, pet::PetPackage, platform::windows::SpineCompositionHost,
};

use super::spine::{SpineDcompRenderer, SpineRenderFrame, SpineRuntime};

const BASE_CHARACTER_HEIGHT_LOGICAL: f32 = 420.0;
const MIN_CHARACTER_WIDTH_LOGICAL: f32 = 190.0;
const MAX_CHARACTER_WIDTH_LOGICAL: f32 = 560.0;

pub struct SpinePresentation {
    runtime: SpineRuntime,
    renderer: SpineDcompRenderer,
    host: SpineCompositionHost,
    current_animation: String,
    last_tick: Instant,
}

impl SpinePresentation {
    pub fn new(
        window: &PetWindow,
        package: &PetPackage,
        behavior: Behavior,
        user_scale: f32,
    ) -> Result<Self, String> {
        let settings = package
            .spine_settings()
            .ok_or("Spine package is missing spine settings")?;
        let skeleton = package
            .spine_skeleton_path()
            .ok_or("Spine package is missing skeleton path")?;
        let atlas = package
            .spine_atlas_path()
            .ok_or("Spine package is missing atlas path")?;

        let mut runtime = SpineRuntime::from_files(&skeleton, &atlas, settings.scale)?;
        runtime.set_skin(&settings.default_skin)?;

        let animation = package
            .spine_animation_name(behavior)
            .or_else(|| package.spine_animation_name(Behavior::Idle))
            .ok_or("Spine package has no idle animation mapping")?
            .to_string();
        runtime.set_animation(0, &animation, true)?;
        runtime.update(0.0);

        let frame = runtime.render_frame()?;
        let logical_size = target_window_size(&frame, user_scale)?;
        window.window().set_size(logical_size);

        let scale_factor = window.window().scale_factor();
        let width = (logical_size.width * scale_factor).round().max(1.0) as u32;
        let height = (logical_size.height * scale_factor).round().max(1.0) as u32;

        let host = SpineCompositionHost::new(&window.window(), width, height)?;
        let mut renderer = SpineDcompRenderer::new(host.hwnd(), &atlas, width, height)?;
        renderer.render(&frame)?;
        host.sync()?;

        window.set_use_sprite(false);
        window.set_use_spine(true);

        Ok(Self {
            runtime,
            renderer,
            host,
            current_animation: animation,
            last_tick: Instant::now(),
        })
    }

    pub fn set_behavior(&mut self, package: &PetPackage, behavior: Behavior) -> Result<(), String> {
        let requested = package
            .spine_animation_name(behavior)
            .or_else(|| package.spine_animation_name(Behavior::Idle))
            .ok_or("Spine package has no animation mapping")?;

        if requested == self.current_animation {
            return Ok(());
        }

        match self.runtime.set_animation(0, requested, true) {
            Ok(()) => {
                self.current_animation.clear();
                self.current_animation.push_str(requested);
                Ok(())
            }
            Err(primary_error) if behavior != Behavior::Idle => {
                let idle = package
                    .spine_animation_name(Behavior::Idle)
                    .ok_or(primary_error.clone())?;
                self.runtime
                    .set_animation(0, idle, true)
                    .map_err(|fallback| {
                        format!("{primary_error}; idle fallback {idle} also failed: {fallback}")
                    })?;
                self.current_animation.clear();
                self.current_animation.push_str(idle);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    pub fn tick(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick).as_secs_f32().min(0.1);
        self.last_tick = now;

        self.host.sync()?;
        self.runtime.update(delta);
        let frame = self.runtime.render_frame()?;
        self.renderer.render(&frame)
    }
}

fn target_window_size(frame: &SpineRenderFrame, user_scale: f32) -> Result<LogicalSize, String> {
    let (min, max) = frame_bounds(frame).ok_or("Spine setup frame has no renderable vertices")?;
    let source_width = (max[0] - min[0]).max(1.0);
    let source_height = (max[1] - min[1]).max(1.0);
    let scale = if user_scale.is_finite() {
        user_scale.clamp(0.6, 1.4)
    } else {
        1.0
    };

    let height = BASE_CHARACTER_HEIGHT_LOGICAL * scale;
    let aspect = (source_width / source_height).clamp(0.35, 1.35);
    let width = (height * aspect).clamp(
        MIN_CHARACTER_WIDTH_LOGICAL * scale,
        MAX_CHARACTER_WIDTH_LOGICAL * scale,
    );

    Ok(LogicalSize::new(width, height))
}

fn frame_bounds(frame: &SpineRenderFrame) -> Option<([f32; 2], [f32; 2])> {
    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    let mut found = false;

    for vertex in frame.batches.iter().flat_map(|batch| batch.vertices.iter()) {
        found = true;
        min[0] = min[0].min(vertex.position[0]);
        min[1] = min[1].min(vertex.position[1]);
        max[0] = max[0].max(vertex.position[0]);
        max[1] = max[1].max(vertex.position[1]);
    }

    found.then_some((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::spine::{SpineBlendMode, SpineRenderBatch, SpineVertex};

    fn frame(width: f32, height: f32) -> SpineRenderFrame {
        SpineRenderFrame {
            batches: vec![SpineRenderBatch {
                texture_page: "page.png".into(),
                slot_name: "body".into(),
                attachment_name: "body".into(),
                blend_mode: SpineBlendMode::Normal,
                vertices: vec![
                    SpineVertex {
                        position: [0.0, 0.0],
                        uv: [0.0, 0.0],
                        light: [1.0; 4],
                        dark: [0.0; 3],
                    },
                    SpineVertex {
                        position: [width, height],
                        uv: [1.0, 1.0],
                        light: [1.0; 4],
                        dark: [0.0; 3],
                    },
                ],
                indices: vec![0, 1, 1],
            }],
        }
    }

    #[test]
    fn target_size_keeps_chibi_character_near_desktop_height() {
        let size = target_window_size(&frame(300.0, 600.0), 1.0).expect("size");
        assert_eq!(size.height, 420.0);
        assert_eq!(size.width, 210.0);
    }

    #[test]
    fn target_size_respects_user_scale() {
        let small = target_window_size(&frame(300.0, 600.0), 0.6).expect("small");
        let large = target_window_size(&frame(300.0, 600.0), 1.4).expect("large");
        assert!(small.height < large.height);
        assert!(small.width < large.width);
    }
}
