#![cfg_attr(not(target_os = "windows"), allow(dead_code, unused_imports))]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("sena_3d_preview currently requires Windows.");
}

#[cfg(target_os = "windows")]
mod windows_preview {
    use std::{
        num::NonZeroIsize,
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };

    use bytemuck::{Pod, Zeroable};
    use glam::{Mat3, Mat4, Vec3};
    use raw_window_handle::{
        RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
    };
    use windows::{
        Win32::{
            Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
            Graphics::Dwm::DwmExtendFrameIntoClientArea,
            System::LibraryLoader::GetModuleHandleW,
            UI::{
                Controls::MARGINS,
                Input::KeyboardAndMouse::VK_ESCAPE,
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG,
                    PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE,
                    ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE,
                    WM_DESTROY, WM_KEYDOWN, WM_QUIT, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
                    WS_POPUP,
                },
            },
        },
        core::w,
    };

    const WIDTH: u32 = 460;
    const HEIGHT: u32 = 560;

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct Vertex {
        position: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
    }

    impl Vertex {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4];

        fn layout() -> wgpu::VertexBufferLayout<'static> {
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &Self::ATTRIBUTES,
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct CameraUniform {
        view_projection: [[f32; 4]; 4],
    }

    struct CpuModel {
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        min: Vec3,
        max: Vec3,
    }

    struct GpuRenderer<'window> {
        surface: wgpu::Surface<'window>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        pipeline: wgpu::RenderPipeline,
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        index_count: u32,
        camera_buffer: wgpu::Buffer,
        camera_bind_group: wgpu::BindGroup,
        started_at: Instant,
    }

    impl GpuRenderer<'static> {
        fn new(hwnd: HWND, width: u32, height: u32, model: CpuModel) -> Result<Self, String> {
            use wgpu::util::DeviceExt;

            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: selected_backends(),
                ..Default::default()
            });

            let window_handle =
                Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).ok_or("invalid HWND")?);
            let display_handle = WindowsDisplayHandle::new();

            let surface = unsafe {
                instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: RawDisplayHandle::Windows(display_handle),
                    raw_window_handle: RawWindowHandle::Win32(window_handle),
                })
            }
            .map_err(|error| format!("failed to create wgpu surface: {error}"))?;

            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
                .ok_or("no compatible GPU adapter found")?;

            let info = adapter.get_info();
            eprintln!(
                "Sena 3D adapter: {} ({:?}, {:?})",
                info.name, info.backend, info.device_type
            );

            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Sena 3D device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            ))
            .map_err(|error| format!("failed to create GPU device: {error}"))?;

            let capabilities = surface.get_capabilities(&adapter);
            let format = capabilities
                .formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .or_else(|| capabilities.formats.first().copied())
                .ok_or("surface has no supported formats")?;
            let alpha_mode = [
                wgpu::CompositeAlphaMode::PreMultiplied,
                wgpu::CompositeAlphaMode::PostMultiplied,
                wgpu::CompositeAlphaMode::Inherit,
            ]
            .into_iter()
            .find(|mode| capabilities.alpha_modes.contains(mode))
            .unwrap_or(wgpu::CompositeAlphaMode::Opaque);

            eprintln!(
                "Sena 3D surface: format={format:?}, alpha={alpha_mode:?}, alpha_modes={:?}",
                capabilities.alpha_modes
            );
            if alpha_mode == wgpu::CompositeAlphaMode::Opaque {
                eprintln!(
                    "Sena 3D transparency unavailable on this backend; production should fall back to Sprite until a transparent composition path is available."
                );
            }

            let present_mode = if capabilities
                .present_modes
                .contains(&wgpu::PresentMode::Fifo)
            {
                wgpu::PresentMode::Fifo
            } else {
                capabilities.present_modes[0]
            };

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);

            let camera = camera_for_bounds(model.min, model.max, width, height, 0.0);
            let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sena camera"),
                contents: bytemuck::bytes_of(&camera),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sena camera layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
            let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sena camera bind group"),
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
            });

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Sena toon shader"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
struct Camera {
    view_projection: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    out.position = camera.view_projection * vec4<f32>(input.position, 1.0);
    out.normal = normalize(input.normal);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(-0.35, 0.70, -0.65));
    let ndl = max(dot(normalize(input.normal), light), 0.0);
    let toon = select(0.70, select(0.86, 1.0, ndl > 0.72), ndl > 0.28);
    let ambient = vec3<f32>(0.14, 0.11, 0.20);
    let rgb = input.color.rgb * (vec3<f32>(toon) + ambient);
    return vec4<f32>(rgb * input.color.a, input.color.a);
}
"#
                    .into(),
                ),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sena 3D pipeline layout"),
                bind_group_layouts: &[&camera_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Sena 3D pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sena vertices"),
                contents: bytemuck::cast_slice(&model.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sena indices"),
                contents: bytemuck::cast_slice(&model.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            Ok(Self {
                surface,
                device,
                queue,
                config,
                pipeline,
                vertex_buffer,
                index_buffer,
                index_count: model.indices.len() as u32,
                camera_buffer,
                camera_bind_group,
                started_at: Instant::now(),
            })
        }

        fn render(&mut self, min: Vec3, max: Vec3) -> Result<(), wgpu::SurfaceError> {
            let elapsed = self.started_at.elapsed().as_secs_f32();
            let camera = camera_for_bounds(
                min,
                max,
                self.config.width,
                self.config.height,
                (elapsed * 0.22).sin() * 0.10,
            );
            self.queue
                .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera));

            let frame = self.surface.get_current_texture()?;
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Sena 3D encoder"),
                });

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Sena 3D pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }

            self.queue.submit(Some(encoder.finish()));
            frame.present();
            Ok(())
        }
    }

    fn selected_backends() -> wgpu::Backends {
        match std::env::var("SENA_WGPU_BACKEND")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "dx12" | "d3d12" => wgpu::Backends::DX12,
            "vulkan" | "vk" => wgpu::Backends::VULKAN,
            _ => wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
        }
    }

    fn camera_for_bounds(min: Vec3, max: Vec3, width: u32, height: u32, yaw: f32) -> CameraUniform {
        let center = (min + max) * 0.5;
        let size = max - min;
        let aspect = width as f32 / height.max(1) as f32;
        // glTF is Y-up. Blender performs this axis conversion during export,
        // so the runtime camera must use Y as vertical and Z as depth.
        let half_height = (size.y * 0.62).max(0.65);
        let half_width = half_height * aspect;
        let distance = size.max_element().max(1.0) * 3.4;
        let eye = center + Vec3::new(yaw.sin() * distance, 0.06, -yaw.cos() * distance);
        let view = Mat4::look_at_rh(eye, center, Vec3::Y);
        let projection = Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.01,
            distance * 3.0,
        );

        CameraUniform {
            view_projection: (projection * view).to_cols_array_2d(),
        }
    }

    fn load_glb(path: &Path) -> Result<CpuModel, String> {
        let gltf = gltf::Gltf::open(path)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
        let blob = gltf
            .blob
            .as_deref()
            .ok_or_else(|| format!("{} does not contain an embedded GLB buffer", path.display()))?;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        let scene = gltf
            .default_scene()
            .or_else(|| gltf.scenes().next())
            .ok_or("GLB has no scene")?;

        for node in scene.nodes() {
            append_node(
                node,
                Mat4::IDENTITY,
                blob,
                &mut vertices,
                &mut indices,
                &mut min,
                &mut max,
            )?;
        }

        if vertices.is_empty() || indices.is_empty() {
            return Err("GLB contains no renderable triangle primitives".into());
        }

        eprintln!(
            "Sena GLB loaded: {} vertices, {} triangles, bounds {:?}..{:?}",
            vertices.len(),
            indices.len() / 3,
            min,
            max
        );

        Ok(CpuModel {
            vertices,
            indices,
            min,
            max,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn append_node(
        node: gltf::Node<'_>,
        parent: Mat4,
        blob: &[u8],
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        min: &mut Vec3,
        max: &mut Vec3,
    ) -> Result<(), String> {
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = parent * local;
        let normal_matrix = Mat3::from_mat4(world).inverse().transpose();

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    continue;
                }

                let reader = primitive.reader(|buffer| match buffer.source() {
                    gltf::buffer::Source::Bin => Some(blob),
                    gltf::buffer::Source::Uri(_) => None,
                });
                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or_else(|| format!("mesh {} primitive has no positions", mesh.index()))?
                    .collect();
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|values| values.collect())
                    .unwrap_or_else(|| vec![[0.0, -1.0, 0.0]; positions.len()]);
                let base_color = primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_factor();
                let vertex_base = vertices.len() as u32;

                for (position, normal) in positions.iter().zip(normals.iter()) {
                    let p = world.transform_point3(Vec3::from_array(*position));
                    let n = (normal_matrix * Vec3::from_array(*normal)).normalize_or_zero();
                    *min = min.min(p);
                    *max = max.max(p);
                    vertices.push(Vertex {
                        position: p.to_array(),
                        normal: n.to_array(),
                        color: base_color,
                    });
                }

                if let Some(read_indices) = reader.read_indices() {
                    indices.extend(read_indices.into_u32().map(|index| vertex_base + index));
                } else {
                    indices.extend((0..positions.len() as u32).map(|index| vertex_base + index));
                }
            }
        }

        for child in node.children() {
            append_node(child, world, blob, vertices, indices, min, max)?;
        }

        Ok(())
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    fn create_window() -> Result<HWND, String> {
        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
        let instance = HINSTANCE(module.0);
        let class_name = w!("Sena3DPreviewWindow");

        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name,
            ..Default::default()
        };
        let atom = unsafe { RegisterClassW(&class) };
        if atom == 0 {
            return Err(format!(
                "RegisterClassW failed: {}",
                windows::core::Error::from_thread()
            ));
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0),
                class_name,
                w!("Sena 3D Preview"),
                WINDOW_STYLE(WS_POPUP.0),
                120,
                120,
                WIDTH as i32,
                HEIGHT as i32,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .map_err(|error| format!("CreateWindowExW failed: {error}"))?;

        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        unsafe {
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }

        Ok(hwnd)
    }

    fn parse_args() -> (PathBuf, Option<u64>) {
        let mut path = PathBuf::from("pets/sena/models/generated/sena_v1.glb");
        let mut max_frames = None;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--frames" => {
                    max_frames = args.next().and_then(|value| value.parse().ok());
                }
                "--model" => {
                    if let Some(value) = args.next() {
                        path = PathBuf::from(value);
                    }
                }
                _ => {}
            }
        }

        (path, max_frames)
    }

    pub fn run() -> Result<(), String> {
        let (model_path, max_frames) = parse_args();
        if !model_path.is_file() {
            return Err(format!(
                "{} not found. Run ./tools/blender/build_sena_v1.ps1 first.",
                model_path.display()
            ));
        }

        let model = load_glb(&model_path)?;
        let bounds = (model.min, model.max);
        let hwnd = create_window()?;
        let mut renderer = GpuRenderer::new(hwnd, WIDTH, HEIGHT, model)?;

        let mut rendered_frames = 0u64;
        let mut message = MSG::default();
        let frame_time = Duration::from_micros(16_667);

        'running: loop {
            let frame_started = Instant::now();

            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                if message.message == WM_QUIT {
                    break 'running;
                }
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            match renderer.render(bounds.0, bounds.1) {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    renderer
                        .surface
                        .configure(&renderer.device, &renderer.config);
                }
                Err(wgpu::SurfaceError::Timeout) => {}
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    return Err("GPU surface ran out of memory".into());
                }
            }

            rendered_frames += 1;
            if max_frames.is_some_and(|limit| rendered_frames >= limit) {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                break;
            }

            if let Some(remaining) = frame_time.checked_sub(frame_started.elapsed()) {
                std::thread::sleep(remaining);
            }
        }

        eprintln!("Sena 3D preview rendered {rendered_frames} frames.");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = windows_preview::run() {
        eprintln!("Sena 3D preview failed: {error}");
        std::process::exit(1);
    }
}
