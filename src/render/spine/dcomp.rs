use std::{
    collections::HashMap,
    mem::size_of,
    path::{Path, PathBuf},
    slice,
};

use bytemuck::{Pod, Zeroable};
use image::ImageReader;
use windows::{
    Win32::{
        Foundation::{HMODULE, HWND},
        Graphics::{
            Direct3D::{
                D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0,
                D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, Fxc::D3DCompile, ID3DBlob,
            },
            Direct3D11::{
                D3D11_BIND_INDEX_BUFFER, D3D11_BIND_SHADER_RESOURCE, D3D11_BIND_VERTEX_BUFFER,
                D3D11_BLEND_DESC, D3D11_BLEND_DEST_COLOR, D3D11_BLEND_INV_SRC_ALPHA,
                D3D11_BLEND_INV_SRC_COLOR, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD, D3D11_BUFFER_DESC,
                D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_INPUT_ELEMENT_DESC,
                D3D11_INPUT_PER_VERTEX_DATA, D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC,
                D3D11_SDK_VERSION, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE_ADDRESS_CLAMP,
                D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_IMMUTABLE, D3D11_VIEWPORT,
                D3D11CreateDevice, ID3D11BlendState, ID3D11Buffer, ID3D11Device,
                ID3D11DeviceContext, ID3D11InputLayout, ID3D11PixelShader, ID3D11RenderTargetView,
                ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
            },
            DirectComposition::{
                DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget,
                IDCompositionVisual,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32G32B32A32_FLOAT,
                    DXGI_SAMPLE_DESC,
                },
                DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter,
                IDXGIDevice, IDXGIFactory2, IDXGIOutput, IDXGISwapChain1,
            },
        },
    },
    core::{BOOL, Interface, PCSTR, s},
};

use super::{SpineBlendMode, SpineRenderFrame};

const SHADER: &[u8] = br#"
struct VsIn {
    float2 position : POSITION;
    float2 uv : TEXCOORD0;
    float4 color : COLOR0;
};

struct VsOut {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD0;
    float4 color : COLOR0;
};

VsOut vs_main(VsIn input) {
    VsOut output;
    output.position = float4(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

Texture2D atlas_texture : register(t0);
SamplerState atlas_sampler : register(s0);

float4 ps_main(VsOut input) : SV_TARGET {
    float4 sampled = atlas_texture.Sample(atlas_sampler, input.uv);
    float4 tint = float4(input.color.rgb * input.color.a, input.color.a);
    return sampled * tint;
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

struct TexturePage {
    _texture: ID3D11Texture2D,
    view: ID3D11ShaderResourceView,
}

pub struct SpineDcompRenderer {
    width: u32,
    height: u32,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    render_target: ID3D11RenderTargetView,
    _dcomp_device: IDCompositionDevice,
    _dcomp_target: IDCompositionTarget,
    _dcomp_visual: IDCompositionVisual,
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    input_layout: ID3D11InputLayout,
    sampler: ID3D11SamplerState,
    normal_blend: ID3D11BlendState,
    additive_blend: ID3D11BlendState,
    multiply_blend: ID3D11BlendState,
    screen_blend: ID3D11BlendState,
    textures: HashMap<String, TexturePage>,
    atlas_dir: PathBuf,
}

impl SpineDcompRenderer {
    pub fn new(hwnd: HWND, atlas_path: &Path, width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("DirectComposition surface size must be non-zero".into());
        }

        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        }
        .map_err(|error| format!("D3D11CreateDevice failed: {error}"))?;

        let device = device.ok_or("D3D11 did not return a device")?;
        let context = context.ok_or("D3D11 did not return an immediate context")?;

        let dxgi_device: IDXGIDevice = device
            .cast()
            .map_err(|error| format!("ID3D11Device -> IDXGIDevice failed: {error}"))?;
        let adapter = unsafe { dxgi_device.GetAdapter() }
            .map_err(|error| format!("IDXGIDevice::GetAdapter failed: {error}"))?;
        let factory: IDXGIFactory2 = unsafe { adapter.GetParent() }
            .map_err(|error| format!("IDXGIAdapter::GetParent<IDXGIFactory2> failed: {error}"))?;

        let swap_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: BOOL(0),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            Flags: 0,
        };

        let swap_chain = unsafe {
            factory.CreateSwapChainForComposition(&device, &swap_desc, None::<&IDXGIOutput>)
        }
        .map_err(|error| format!("CreateSwapChainForComposition failed: {error}"))?;
        let render_target = create_render_target(&device, &swap_chain)?;

        let dcomp_device: IDCompositionDevice =
            unsafe { DCompositionCreateDevice(&dxgi_device) }
                .map_err(|error| format!("DCompositionCreateDevice failed: {error}"))?;
        let dcomp_target = unsafe { dcomp_device.CreateTargetForHwnd(hwnd, true) }
            .map_err(|error| format!("CreateTargetForHwnd failed: {error}"))?;
        let dcomp_visual = unsafe { dcomp_device.CreateVisual() }
            .map_err(|error| format!("IDCompositionDevice::CreateVisual failed: {error}"))?;
        unsafe {
            dcomp_visual
                .SetContent(&swap_chain)
                .map_err(|error| format!("IDCompositionVisual::SetContent failed: {error}"))?;
            dcomp_target
                .SetRoot(&dcomp_visual)
                .map_err(|error| format!("IDCompositionTarget::SetRoot failed: {error}"))?;
            dcomp_device
                .Commit()
                .map_err(|error| format!("IDCompositionDevice::Commit failed: {error}"))?;
        }

        let vertex_blob = compile_shader(SHADER, s!("vs_main"), s!("vs_4_0"))?;
        let pixel_blob = compile_shader(SHADER, s!("ps_main"), s!("ps_4_0"))?;
        let vertex_bytes = blob_bytes(&vertex_blob);
        let pixel_bytes = blob_bytes(&pixel_blob);

        let mut vertex_shader = None;
        unsafe { device.CreateVertexShader(vertex_bytes, None, Some(&mut vertex_shader)) }
            .map_err(|error| format!("CreateVertexShader failed: {error}"))?;
        let vertex_shader = vertex_shader.ok_or("D3D11 did not return a vertex shader")?;

        let mut pixel_shader = None;
        unsafe { device.CreatePixelShader(pixel_bytes, None, Some(&mut pixel_shader)) }
            .map_err(|error| format!("CreatePixelShader failed: {error}"))?;
        let pixel_shader = pixel_shader.ok_or("D3D11 did not return a pixel shader")?;

        let layout_desc = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 8,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("COLOR"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 16,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        let mut input_layout = None;
        unsafe { device.CreateInputLayout(&layout_desc, vertex_bytes, Some(&mut input_layout)) }
            .map_err(|error| format!("CreateInputLayout failed: {error}"))?;
        let input_layout = input_layout.ok_or("D3D11 did not return an input layout")?;

        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MinLOD: 0.0,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let mut sampler = None;
        unsafe { device.CreateSamplerState(&sampler_desc, Some(&mut sampler)) }
            .map_err(|error| format!("CreateSamplerState failed: {error}"))?;
        let sampler = sampler.ok_or("D3D11 did not return a sampler")?;

        let normal_blend = create_pma_blend_state(&device, SpineBlendMode::Normal)?;
        let additive_blend = create_pma_blend_state(&device, SpineBlendMode::Additive)?;
        let multiply_blend = create_pma_blend_state(&device, SpineBlendMode::Multiply)?;
        let screen_blend = create_pma_blend_state(&device, SpineBlendMode::Screen)?;

        Ok(Self {
            width,
            height,
            device,
            context,
            swap_chain,
            render_target,
            _dcomp_device: dcomp_device,
            _dcomp_target: dcomp_target,
            _dcomp_visual: dcomp_visual,
            vertex_shader,
            pixel_shader,
            input_layout,
            sampler,
            normal_blend,
            additive_blend,
            multiply_blend,
            screen_blend,
            textures: HashMap::new(),
            atlas_dir: atlas_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        })
    }

    pub fn render(&mut self, frame: &SpineRenderFrame) -> Result<(), String> {
        let bounds = frame_bounds(frame).ok_or("Spine render frame has no vertices")?;

        unsafe {
            self.context
                .OMSetRenderTargets(Some(&[Some(self.render_target.clone())]), None);
            self.context
                .ClearRenderTargetView(&self.render_target, &[0.0, 0.0, 0.0, 0.0]);
            self.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.width as f32,
                Height: self.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            self.context.IASetInputLayout(&self.input_layout);
            self.context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.context.VSSetShader(&self.vertex_shader, None);
            self.context.PSSetShader(&self.pixel_shader, None);
            self.context
                .PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
        }

        for batch in &frame.batches {
            let texture_view = self.texture_view(&batch.texture_page)?;
            let gpu_vertices = batch
                .vertices
                .iter()
                .map(|vertex| GpuVertex {
                    position: to_ndc(vertex.position, bounds, self.width, self.height),
                    uv: vertex.uv,
                    color: vertex.light,
                })
                .collect::<Vec<_>>();

            let vertex_buffer = create_immutable_buffer(
                &self.device,
                &gpu_vertices,
                D3D11_BIND_VERTEX_BUFFER.0 as u32,
            )?;
            let index_buffer = create_immutable_buffer(
                &self.device,
                &batch.indices,
                D3D11_BIND_INDEX_BUFFER.0 as u32,
            )?;

            let buffers = [Some(vertex_buffer)];
            let stride = size_of::<GpuVertex>() as u32;
            let offset = 0u32;
            let blend = match batch.blend_mode {
                SpineBlendMode::Normal => &self.normal_blend,
                SpineBlendMode::Additive => &self.additive_blend,
                SpineBlendMode::Multiply => &self.multiply_blend,
                SpineBlendMode::Screen => &self.screen_blend,
            };

            unsafe {
                self.context.IASetVertexBuffers(
                    0,
                    1,
                    Some(buffers.as_ptr()),
                    Some(&stride),
                    Some(&offset),
                );
                self.context
                    .IASetIndexBuffer(&index_buffer, DXGI_FORMAT_R16_UINT, 0);
                self.context
                    .PSSetShaderResources(0, Some(&[Some(texture_view)]));
                self.context.OMSetBlendState(blend, None, u32::MAX);
                self.context.DrawIndexed(batch.indices.len() as u32, 0, 0);
            }
        }

        unsafe { self.swap_chain.Present(1, DXGI_PRESENT(0)) }
            .ok()
            .map_err(|error| format!("IDXGISwapChain1::Present failed: {error}"))
    }

    fn texture_view(&mut self, page_name: &str) -> Result<ID3D11ShaderResourceView, String> {
        if !self.textures.contains_key(page_name) {
            let page = load_texture(&self.device, &self.atlas_dir.join(page_name))?;
            self.textures.insert(page_name.to_string(), page);
        }
        Ok(self
            .textures
            .get(page_name)
            .expect("texture was inserted immediately above")
            .view
            .clone())
    }
}

fn create_render_target(
    device: &ID3D11Device,
    swap_chain: &IDXGISwapChain1,
) -> Result<ID3D11RenderTargetView, String> {
    let back_buffer: ID3D11Texture2D = unsafe { swap_chain.GetBuffer(0) }
        .map_err(|error| format!("IDXGISwapChain1::GetBuffer failed: {error}"))?;
    let mut render_target = None;
    unsafe { device.CreateRenderTargetView(&back_buffer, None, Some(&mut render_target)) }
        .map_err(|error| format!("CreateRenderTargetView failed: {error}"))?;
    render_target.ok_or_else(|| "D3D11 did not return a render target".into())
}

fn create_pma_blend_state(
    device: &ID3D11Device,
    mode: SpineBlendMode,
) -> Result<ID3D11BlendState, String> {
    let (src_rgb, dst_rgb) = match mode {
        SpineBlendMode::Normal => (D3D11_BLEND_ONE, D3D11_BLEND_INV_SRC_ALPHA),
        SpineBlendMode::Additive => (D3D11_BLEND_ONE, D3D11_BLEND_ONE),
        SpineBlendMode::Multiply => (D3D11_BLEND_DEST_COLOR, D3D11_BLEND_INV_SRC_ALPHA),
        SpineBlendMode::Screen => (D3D11_BLEND_ONE, D3D11_BLEND_INV_SRC_COLOR),
    };

    let mut desc = D3D11_BLEND_DESC::default();
    desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
        BlendEnable: BOOL(1),
        SrcBlend: src_rgb,
        DestBlend: dst_rgb,
        BlendOp: D3D11_BLEND_OP_ADD,
        SrcBlendAlpha: D3D11_BLEND_ONE,
        DestBlendAlpha: match mode {
            SpineBlendMode::Additive => D3D11_BLEND_ONE,
            SpineBlendMode::Normal | SpineBlendMode::Multiply | SpineBlendMode::Screen => {
                D3D11_BLEND_INV_SRC_ALPHA
            }
        },
        BlendOpAlpha: D3D11_BLEND_OP_ADD,
        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };

    let mut state = None;
    unsafe { device.CreateBlendState(&desc, Some(&mut state)) }
        .map_err(|error| format!("CreateBlendState failed: {error}"))?;
    state.ok_or_else(|| "D3D11 did not return a blend state".into())
}

fn create_immutable_buffer<T: Pod>(
    device: &ID3D11Device,
    values: &[T],
    bind_flags: u32,
) -> Result<ID3D11Buffer, String> {
    let byte_width = values
        .len()
        .checked_mul(size_of::<T>())
        .and_then(|bytes| u32::try_from(bytes).ok())
        .ok_or("D3D11 buffer is too large")?;
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: byte_width,
        Usage: D3D11_USAGE_IMMUTABLE,
        BindFlags: bind_flags,
        ..Default::default()
    };
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: values.as_ptr().cast(),
        ..Default::default()
    };
    let mut buffer = None;
    unsafe { device.CreateBuffer(&desc, Some(&initial), Some(&mut buffer)) }
        .map_err(|error| format!("CreateBuffer failed: {error}"))?;
    buffer.ok_or_else(|| "D3D11 did not return a buffer".into())
}

fn load_texture(device: &ID3D11Device, path: &Path) -> Result<TexturePage, String> {
    let image = ImageReader::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?
        .decode()
        .map_err(|error| format!("failed to decode {}: {error}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        ..Default::default()
    };

    let bgra = rgba
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]])
        .collect::<Vec<_>>();
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: bgra.as_ptr().cast(),
        SysMemPitch: width * 4,
        ..Default::default()
    };

    let mut texture = None;
    unsafe { device.CreateTexture2D(&desc, Some(&initial), Some(&mut texture)) }
        .map_err(|error| format!("CreateTexture2D failed for {}: {error}", path.display()))?;
    let texture = texture.ok_or("D3D11 did not return a texture")?;

    let mut view = None;
    unsafe { device.CreateShaderResourceView(&texture, None, Some(&mut view)) }.map_err(
        |error| {
            format!(
                "CreateShaderResourceView failed for {}: {error}",
                path.display()
            )
        },
    )?;
    let view = view.ok_or("D3D11 did not return a shader-resource view")?;

    Ok(TexturePage {
        _texture: texture,
        view,
    })
}

fn compile_shader(source: &[u8], entry: PCSTR, target: PCSTR) -> Result<ID3DBlob, String> {
    let mut code = None;
    let mut errors = None;
    let result = unsafe {
        D3DCompile(
            source.as_ptr().cast(),
            source.len(),
            PCSTR::null(),
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };

    if let Err(error) = result {
        let details = errors
            .as_ref()
            .map(blob_bytes)
            .map(String::from_utf8_lossy)
            .map(|text| text.into_owned())
            .unwrap_or_else(|| error.to_string());
        return Err(format!("D3DCompile failed: {details}"));
    }

    code.ok_or_else(|| "D3DCompile returned no bytecode".into())
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    unsafe { slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize()) }
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

fn to_ndc(
    position: [f32; 2],
    bounds: ([f32; 2], [f32; 2]),
    width_px: u32,
    height_px: u32,
) -> [f32; 2] {
    let (min, max) = bounds;
    let width = (max[0] - min[0]).max(1.0);
    let height = (max[1] - min[1]).max(1.0);
    let center_x = (min[0] + max[0]) * 0.5;
    let center_y = (min[1] + max[1]) * 0.5;
    let pixels_per_unit = (0.88 * width_px as f32 / width).min(0.88 * height_px as f32 / height);

    [
        (position[0] - center_x) * pixels_per_unit * 2.0 / width_px as f32,
        -(position[1] - center_y) * pixels_per_unit * 2.0 / height_px as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndc_mapping_keeps_bounds_inside_preview_margin() {
        let bounds = ([-100.0, -200.0], [100.0, 200.0]);
        let bottom_left = to_ndc([-100.0, -200.0], bounds, 400, 800);
        let top_right = to_ndc([100.0, 200.0], bounds, 400, 800);

        assert!(bottom_left[0] >= -0.9);
        assert!(bottom_left[1] <= 0.9);
        assert!(top_right[0] <= 0.9);
        assert!(top_right[1] >= -0.9);
    }
}
