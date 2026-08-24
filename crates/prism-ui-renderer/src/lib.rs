//! Shared `wgpu` renderer used by WebGPU, Metal, and Vulkan hosts.

use std::{borrow::Cow, num::NonZeroU64};

use bytemuck::{Pod, Zeroable};
use thiserror::Error;

mod optics;

pub use optics::{
    BASIC_PRISM_PLANES, MAX_LIGHTS, MAX_SOURCE_SAMPLES, OPTICAL_PATH_BUFFER_FLOATS, OpticalFrame,
    OpticalPathLookup, SPECTRAL_ANCHOR_COUNT, optical_light_path_texel_offset,
    optical_material_path_texel_offset,
};

const INTERNAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const FRAME_UNIFORM_SIZE: u64 = std::mem::size_of::<FrameUniforms>() as u64;
const OPTICAL_BUFFER_SIZE: u64 = (OPTICAL_PATH_BUFFER_FLOATS * std::mem::size_of::<f32>()) as u64;
const FRAME_BUFFER_ALIGNMENT: u64 = 256;
const LIGHT_UNIFORM_OFFSET: u64 = 0;
const MATERIAL_UNIFORM_OFFSET: u64 = aligned_offset(LIGHT_UNIFORM_OFFSET + FRAME_UNIFORM_SIZE);
const OPTICAL_BUFFER_OFFSET: u64 = aligned_offset(MATERIAL_UNIFORM_OFFSET + FRAME_UNIFORM_SIZE);
const COMPOSITE_PARAMETERS_OFFSET: u64 =
    aligned_offset(OPTICAL_BUFFER_OFFSET + OPTICAL_BUFFER_SIZE);
const FRAME_BUFFER_SIZE: u64 =
    COMPOSITE_PARAMETERS_OFFSET + std::mem::size_of::<CompositeParameters>() as u64;
const PRISM_OPTICAL_WGSL: &str = include_str!(concat!(env!("OUT_DIR"), "/prism-object.wgsl"));

const fn aligned_offset(value: u64) -> u64 {
    value.div_ceil(FRAME_BUFFER_ALIGNMENT) * FRAME_BUFFER_ALIGNMENT
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FrameUniforms {
    pub resolution: [f32; 2],
    pub phase: f32,
    pub light_phase: f32,
    pub light_count: i32,
    pub render_layer: i32,
    pub ior: f32,
    pub dispersion: f32,
    pub transmission: f32,
    pub light_throughput: f32,
    pub material_opacity_scale: f32,
    pub roughness: f32,
    pub fresnel_strength: f32,
    pub sheen_strength: f32,
    pub sheen_width: f32,
    pub sheen_chroma: f32,
    pub visibility: f32,
    pub bevel: f32,
    pub _padding_before_planes: [f32; 2],
    pub prism_planes: [[f32; 4]; 20],
    pub prism_plane_count: i32,
    pub shadow_strength: f32,
    pub light_radius: f32,
    pub source_size: f32,
    pub source_divergence: f32,
    pub source_halo: f32,
    pub source_sample_count: i32,
    pub rayleigh_mix: f32,
    pub caustic_halo: f32,
    pub turbulence_strength: f32,
    pub turbulence_speed: f32,
    pub optical_time: f32,
    pub incident_strength: f32,
    pub scattering_strength: f32,
    pub scattering_falloff: f32,
    pub spectral_fan_reach: f32,
    pub beam_width: f32,
    pub _padding_before_transition: [f32; 3],
    pub transition_geometry: [f32; 4],
    pub transition_appearance: [f32; 4],
    pub icon_size: f32,
    pub viewport_scale: f32,
    pub prism_scale: f32,
    pub transition_prism_scale: f32,
    pub spinner_morph: [f32; 4],
    pub spinner_material_morph: [f32; 2],
    pub spinner_morph_scale: f32,
    pub spinner_frame_size: f32,
    pub css_pixel_ratio: f32,
    pub spinner_morph_match_phase: f32,
    pub _padding_before_pose: [f32; 2],
    pub object_pose_quaternion: [f32; 4],
    pub object_pose_override: f32,
    pub spinner_crease_morph: f32,
    pub material_spectral_sample_count: i32,
    pub _padding_before_normalization: f32,
    pub material_spectral_normalization: [f32; 3],
    pub dark_mode: f32,
    pub environment_luminance: f32,
    pub environment_light_mix: f32,
    pub render_prism_scale: f32,
    pub scaled_scattering_falloff: f32,
    pub icon_edge_color: [f32; 4],
    pub object_projection: [f32; 4],
}

impl Default for FrameUniforms {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct CompositeParameters {
    hdr_headroom: f32,
    _padding: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderTargetSize {
    pub width: u32,
    pub height: u32,
}

pub fn resolve_render_targets(
    width: u32,
    height: u32,
    light_scale: f32,
    material_scale: f32,
) -> (RenderTargetSize, RenderTargetSize) {
    let scaled =
        |value: u32, scale: f32| ((value as f32 * scale.clamp(0.0625, 1.0)).round() as u32).max(1);
    (
        RenderTargetSize {
            width: scaled(width, material_scale),
            height: scaled(height, material_scale),
        },
        RenderTargetSize {
            width: scaled(width, light_scale),
            height: scaled(height, light_scale),
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScissorRectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderFrame<'a> {
    pub uniforms: FrameUniforms,
    pub optical_paths: &'a [f32],
    pub output_size: RenderTargetSize,
    pub light_resolution_scale: f32,
    pub material_resolution_scale: f32,
    pub light_source_sample_count: i32,
    pub material_source_sample_count: i32,
    pub material_scissor: Option<ScissorRectangle>,
    pub render_light: bool,
    pub hdr_headroom: f32,
}

#[derive(Clone, Copy, Debug)]
struct RenderPlan {
    render_light: bool,
    render_hdr: bool,
}

impl RenderPlan {
    const fn for_output(
        render_light: bool,
        output_format: wgpu::TextureFormat,
        hdr_headroom: f32,
    ) -> Self {
        Self {
            render_light,
            render_hdr: matches!(output_format, wgpu::TextureFormat::Rgba16Float)
                && hdr_headroom > 1.0001,
        }
    }

    const fn renders_light(self) -> bool {
        self.render_light
    }
    const fn renders_hdr(self) -> bool {
        self.render_hdr
    }
    const fn composites_material_directly(self) -> bool {
        !self.render_hdr
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("expected {OPTICAL_PATH_BUFFER_FLOATS} optical-path floats, received {0}")]
    InvalidOpticalPathLength(usize),
}

struct RenderTargets {
    material_size: RenderTargetSize,
    light_size: RenderTargetSize,
    _light_base: wgpu::Texture,
    _light_gain: wgpu::Texture,
    _material_base: wgpu::Texture,
    _material_gain: wgpu::Texture,
    light_base_view: wgpu::TextureView,
    light_gain_view: wgpu::TextureView,
    material_base_view: wgpu::TextureView,
    material_gain_view: wgpu::TextureView,
    composite_bind_group: wgpu::BindGroup,
}

pub struct PrismRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    light_pipeline_sdr: wgpu::RenderPipeline,
    light_pipeline_hdr: Option<wgpu::RenderPipeline>,
    material_pipeline_sdr: wgpu::RenderPipeline,
    material_pipeline_direct_sdr: wgpu::RenderPipeline,
    material_pipeline_hdr: Option<wgpu::RenderPipeline>,
    composite_pipeline_sdr: wgpu::RenderPipeline,
    composite_pipeline_sdr_material: wgpu::RenderPipeline,
    composite_pipeline_hdr: Option<wgpu::RenderPipeline>,
    output_format: wgpu::TextureFormat,
    frame_buffer: wgpu::Buffer,
    frame_upload: Box<[u8]>,
    light_bind_group: wgpu::BindGroup,
    material_bind_group: wgpu::BindGroup,
    composite_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    targets: Option<RenderTargets>,
}

impl PrismRenderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let frame_buffer = uniform_buffer(&device, "Prism frame data", FRAME_BUFFER_SIZE);

        let optical_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Prism optical bind group layout"),
                entries: &[
                    uniform_layout_entry(0, FRAME_UNIFORM_SIZE),
                    uniform_layout_entry(1, OPTICAL_BUFFER_SIZE),
                ],
            });
        let optical_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Prism optical pipeline layout"),
                bind_group_layouts: &[Some(&optical_bind_group_layout)],
                immediate_size: 0,
            });
        let fullscreen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prism fullscreen vertex shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "shaders/fullscreen.wgsl"
            ))),
        });
        let light_shader_sdr = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prism SDR light shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(specialized_optical_shader(0, false))),
        });
        let material_shader_sdr = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prism SDR material shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(specialized_optical_shader(1, false))),
        });
        let light_pipeline_sdr = optical_pipeline(
            &device,
            &optical_pipeline_layout,
            &fullscreen_shader,
            &light_shader_sdr,
            "Prism SDR light pipeline",
            false,
        );
        let material_pipeline_sdr = optical_pipeline(
            &device,
            &optical_pipeline_layout,
            &fullscreen_shader,
            &material_shader_sdr,
            "Prism SDR material pipeline",
            false,
        );
        let material_pipeline_direct_sdr = direct_material_pipeline(
            &device,
            &optical_pipeline_layout,
            &fullscreen_shader,
            &material_shader_sdr,
            output_format,
        );
        let hdr_capable = matches!(output_format, wgpu::TextureFormat::Rgba16Float);
        let light_shader_hdr = hdr_capable.then(|| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Prism HDR light shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(specialized_optical_shader(0, true))),
            })
        });
        let material_shader_hdr = hdr_capable.then(|| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Prism HDR material shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(specialized_optical_shader(1, true))),
            })
        });
        let light_pipeline_hdr = hdr_capable.then(|| {
            optical_pipeline(
                &device,
                &optical_pipeline_layout,
                &fullscreen_shader,
                light_shader_hdr.as_ref().unwrap(),
                "Prism HDR light pipeline",
                true,
            )
        });
        let material_pipeline_hdr = hdr_capable.then(|| {
            optical_pipeline(
                &device,
                &optical_pipeline_layout,
                &fullscreen_shader,
                material_shader_hdr.as_ref().unwrap(),
                "Prism HDR material pipeline",
                true,
            )
        });
        let light_bind_group = optical_bind_group(
            &device,
            &optical_bind_group_layout,
            "Prism light bind group",
            &frame_buffer,
            LIGHT_UNIFORM_OFFSET,
        );
        let material_bind_group = optical_bind_group(
            &device,
            &optical_bind_group_layout,
            "Prism material bind group",
            &frame_buffer,
            MATERIAL_UNIFORM_OFFSET,
        );

        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Prism composite bind group layout"),
                entries: &[
                    texture_layout_entry(0),
                    texture_layout_entry(1),
                    texture_layout_entry(2),
                    texture_layout_entry(3),
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    uniform_layout_entry(5, std::mem::size_of::<CompositeParameters>() as u64),
                ],
            });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Prism composite pipeline layout"),
                bind_group_layouts: &[Some(&composite_bind_group_layout)],
                immediate_size: 0,
            });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prism composite shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/composite.wgsl"))),
        });
        let composite_pipeline_sdr = composite_pipeline(
            &device,
            &composite_pipeline_layout,
            &fullscreen_shader,
            &composite_shader,
            output_format,
            "Prism SDR composite pipeline",
            false,
            false,
        );
        let composite_pipeline_sdr_material = composite_pipeline(
            &device,
            &composite_pipeline_layout,
            &fullscreen_shader,
            &composite_shader,
            output_format,
            "Prism scaled SDR composite pipeline",
            false,
            true,
        );
        let composite_pipeline_hdr = hdr_capable.then(|| {
            composite_pipeline(
                &device,
                &composite_pipeline_layout,
                &fullscreen_shader,
                &composite_shader,
                output_format,
                "Prism HDR composite pipeline",
                true,
                true,
            )
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Prism linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            device,
            queue,
            light_pipeline_sdr,
            light_pipeline_hdr,
            material_pipeline_sdr,
            material_pipeline_direct_sdr,
            material_pipeline_hdr,
            composite_pipeline_sdr,
            composite_pipeline_sdr_material,
            composite_pipeline_hdr,
            output_format,
            frame_buffer,
            frame_upload: vec![0; FRAME_BUFFER_SIZE as usize].into_boxed_slice(),
            light_bind_group,
            material_bind_group,
            composite_bind_group_layout,
            sampler,
            targets: None,
        }
    }

    pub fn render(
        &mut self,
        output: &wgpu::TextureView,
        frame: &RenderFrame<'_>,
    ) -> Result<(), RenderError> {
        if frame.optical_paths.len() != OPTICAL_PATH_BUFFER_FLOATS {
            return Err(RenderError::InvalidOpticalPathLength(
                frame.optical_paths.len(),
            ));
        }
        let (material_size, light_size) = resolve_render_targets(
            frame.output_size.width,
            frame.output_size.height,
            frame.light_resolution_scale,
            frame.material_resolution_scale,
        );
        self.ensure_targets(material_size, light_size);
        let targets = self.targets.as_ref().unwrap();
        let plan =
            RenderPlan::for_output(frame.render_light, self.output_format, frame.hdr_headroom);
        let light_pipeline = if plan.renders_hdr() {
            self.light_pipeline_hdr.as_ref().unwrap()
        } else {
            &self.light_pipeline_sdr
        };
        let material_pipeline = if plan.renders_hdr() {
            self.material_pipeline_hdr.as_ref().unwrap()
        } else {
            &self.material_pipeline_sdr
        };
        let direct_material =
            plan.composites_material_directly() && material_size == frame.output_size;
        let composite_pipeline = if plan.renders_hdr() {
            self.composite_pipeline_hdr.as_ref().unwrap()
        } else if direct_material {
            &self.composite_pipeline_sdr
        } else {
            &self.composite_pipeline_sdr_material
        };

        let transition_prism_scale = if frame.uniforms.transition_prism_scale > 0.0 {
            frame.uniforms.transition_prism_scale
        } else {
            let icon_scale = (frame.uniforms.icon_size / 144.0 * 1.005).max(0.001);
            let prism_scale = frame.uniforms.prism_scale.max(0.001);
            let amount = frame.uniforms.transition_geometry[0];
            ((1.0 - amount) * icon_scale.ln() + amount * prism_scale.ln()).exp()
                * frame.uniforms.spinner_morph_scale.max(0.001)
        };
        let render_prism_scale = if frame.uniforms.render_prism_scale > 0.0 {
            frame.uniforms.render_prism_scale
        } else {
            frame.uniforms.prism_scale * frame.uniforms.spinner_morph_scale.max(0.001)
        };
        let environment_light_mix = smoothstep(
            0.18,
            0.82,
            frame.uniforms.environment_luminance.clamp(0.0, 1.0),
        );
        let mut light_uniforms = frame.uniforms;
        light_uniforms.transition_prism_scale = transition_prism_scale;
        light_uniforms.render_prism_scale = render_prism_scale;
        light_uniforms.environment_light_mix = environment_light_mix;
        light_uniforms.scaled_scattering_falloff =
            frame.uniforms.scattering_falloff * render_prism_scale;
        light_uniforms.resolution = [light_size.width as f32, light_size.height as f32];
        light_uniforms.render_layer = 0;
        light_uniforms.source_sample_count = frame.light_source_sample_count;
        let mut material_uniforms = frame.uniforms;
        material_uniforms.transition_prism_scale = transition_prism_scale;
        material_uniforms.render_prism_scale = render_prism_scale;
        material_uniforms.environment_light_mix = environment_light_mix;
        material_uniforms.scaled_scattering_falloff =
            frame.uniforms.scattering_falloff * render_prism_scale;
        material_uniforms.resolution = [material_size.width as f32, material_size.height as f32];
        material_uniforms.render_layer = 1;
        material_uniforms.source_sample_count = frame.material_source_sample_count;
        let composite_parameters = CompositeParameters {
            hdr_headroom: frame.hdr_headroom.clamp(1.0, 2.5),
            _padding: [0.0; 3],
        };
        self.frame_upload
            [LIGHT_UNIFORM_OFFSET as usize..(LIGHT_UNIFORM_OFFSET + FRAME_UNIFORM_SIZE) as usize]
            .copy_from_slice(bytemuck::bytes_of(&light_uniforms));
        self.frame_upload[MATERIAL_UNIFORM_OFFSET as usize
            ..(MATERIAL_UNIFORM_OFFSET + FRAME_UNIFORM_SIZE) as usize]
            .copy_from_slice(bytemuck::bytes_of(&material_uniforms));
        self.frame_upload[OPTICAL_BUFFER_OFFSET as usize
            ..(OPTICAL_BUFFER_OFFSET + OPTICAL_BUFFER_SIZE) as usize]
            .copy_from_slice(bytemuck::cast_slice(frame.optical_paths));
        self.frame_upload[COMPOSITE_PARAMETERS_OFFSET as usize
            ..(COMPOSITE_PARAMETERS_OFFSET + std::mem::size_of::<CompositeParameters>() as u64)
                as usize]
            .copy_from_slice(bytemuck::bytes_of(&composite_parameters));
        self.queue
            .write_buffer(&self.frame_buffer, 0, &self.frame_upload);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Prism frame"),
            });
        if plan.renders_light() {
            self.record_optical_pass(
                &mut encoder,
                "Prism light pass",
                &targets.light_base_view,
                plan.renders_hdr().then_some(&targets.light_gain_view),
                &self.light_bind_group,
                light_pipeline,
                None,
            );
        } else {
            clear_pair(
                &mut encoder,
                "Prism clear light",
                &targets.light_base_view,
                plan.renders_hdr().then_some(&targets.light_gain_view),
            );
        }
        if !direct_material {
            self.record_optical_pass(
                &mut encoder,
                "Prism material pass",
                &targets.material_base_view,
                plan.renders_hdr().then_some(&targets.material_gain_view),
                &self.material_bind_group,
                material_pipeline,
                frame.material_scissor,
            );
        }
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Prism composite pass"),
                color_attachments: &color_attachments,
                ..Default::default()
            });
            pass.set_pipeline(composite_pipeline);
            pass.set_bind_group(0, &targets.composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if direct_material {
            self.record_direct_material_pass(&mut encoder, output, frame.material_scissor);
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    pub fn present(&self, surface_texture: wgpu::SurfaceTexture) {
        self.queue.present(surface_texture);
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    fn ensure_targets(&mut self, material_size: RenderTargetSize, light_size: RenderTargetSize) {
        let unchanged = self.targets.as_ref().is_some_and(|targets| {
            targets.material_size == material_size && targets.light_size == light_size
        });
        if unchanged {
            return;
        }
        let light_base = render_texture(&self.device, "Prism light base", light_size);
        let light_gain = render_texture(&self.device, "Prism light gain", light_size);
        let material_base = render_texture(&self.device, "Prism material base", material_size);
        let material_gain = render_texture(&self.device, "Prism material gain", material_size);
        let light_base_view = light_base.create_view(&Default::default());
        let light_gain_view = light_gain.create_view(&Default::default());
        let material_base_view = material_base.create_view(&Default::default());
        let material_gain_view = material_gain.create_view(&Default::default());
        let composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Prism composite bind group"),
            layout: &self.composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&light_base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&light_gain_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&material_base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&material_gain_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffer_binding(
                        &self.frame_buffer,
                        COMPOSITE_PARAMETERS_OFFSET,
                        std::mem::size_of::<CompositeParameters>() as u64,
                    ),
                },
            ],
        });
        self.targets = Some(RenderTargets {
            material_size,
            light_size,
            _light_base: light_base,
            _light_gain: light_gain,
            _material_base: material_base,
            _material_gain: material_gain,
            light_base_view,
            light_gain_view,
            material_base_view,
            material_gain_view,
            composite_bind_group,
        });
    }

    fn record_optical_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &'static str,
        base: &wgpu::TextureView,
        gain: Option<&wgpu::TextureView>,
        bind_group: &wgpu::BindGroup,
        pipeline: &wgpu::RenderPipeline,
        scissor: Option<ScissorRectangle>,
    ) {
        let color_attachments = [
            Some(wgpu::RenderPassColorAttachment {
                view: base,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            }),
            gain.map(|view| wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &color_attachments,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        if let Some(scissor) = scissor {
            pass.set_scissor_rect(
                scissor.x,
                scissor.y,
                scissor.width.max(1),
                scissor.height.max(1),
            );
        }
        pass.draw(0..3, 0..1);
    }

    fn record_direct_material_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        scissor: Option<ScissorRectangle>,
    ) {
        let color_attachments = [
            Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            None,
        ];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Prism direct SDR material pass"),
            color_attachments: &color_attachments,
            ..Default::default()
        });
        pass.set_pipeline(&self.material_pipeline_direct_sdr);
        pass.set_bind_group(0, &self.material_bind_group, &[]);
        if let Some(scissor) = scissor {
            pass.set_scissor_rect(
                scissor.x,
                scissor.y,
                scissor.width.max(1),
                scissor.height.max(1),
            );
        }
        pass.draw(0..3, 0..1);
    }
}

fn uniform_buffer(device: &wgpu::Device, label: &'static str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let amount = ((value - start) / (end - start)).clamp(0.0, 1.0);
    amount * amount * (3.0 - 2.0 * amount)
}

fn uniform_layout_entry(binding: u32, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(size),
        },
        count: None,
    }
}

fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn optical_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &'static str,
    frame_buffer: &wgpu::Buffer,
    uniform_offset: u64,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_binding(frame_buffer, uniform_offset, FRAME_UNIFORM_SIZE),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_binding(frame_buffer, OPTICAL_BUFFER_OFFSET, OPTICAL_BUFFER_SIZE),
            },
        ],
    })
}

fn buffer_binding(buffer: &wgpu::Buffer, offset: u64, size: u64) -> wgpu::BindingResource<'_> {
    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        buffer,
        offset,
        size: NonZeroU64::new(size),
    })
}

fn vertex_state(module: &wgpu::ShaderModule) -> wgpu::VertexState<'_> {
    wgpu::VertexState {
        module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        buffers: &[],
    }
}

fn specialized_optical_shader(render_layer: i32, hdr: bool) -> String {
    assert!(matches!(render_layer, 0 | 1));
    let shader = PRISM_OPTICAL_WGSL.replace("global.uRenderLayer", &format!("{render_layer}i"));
    if hdr {
        shader
    } else {
        replace_wgsl_function_body(shader, "encodeHdrGain", "    return vec4(0f);")
    }
}

fn replace_wgsl_function_body(mut shader: String, name: &str, body: &str) -> String {
    let function_start = shader
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("WGSL function {name} is missing"));
    let opening_brace = function_start
        + shader[function_start..]
            .find('{')
            .unwrap_or_else(|| panic!("WGSL function {name} has no body"));
    let mut depth = 0usize;
    let mut closing_brace = None;
    for (relative_index, character) in shader[opening_brace..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    closing_brace = Some(opening_brace + relative_index);
                    break;
                }
            }
            _ => {}
        }
    }
    let closing_brace = closing_brace.unwrap_or_else(|| panic!("WGSL function {name} is unclosed"));
    shader.replace_range(opening_brace..=closing_brace, &format!("{{\n{body}\n}}"));
    shader
}

fn optical_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vertex_shader: &wgpu::ShaderModule,
    fragment_shader: &wgpu::ShaderModule,
    label: &'static str,
    hdr: bool,
) -> wgpu::RenderPipeline {
    let targets = [
        Some(wgpu::ColorTargetState {
            format: INTERNAL_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }),
        hdr.then_some(wgpu::ColorTargetState {
            format: INTERNAL_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: vertex_state(vertex_shader),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: fragment_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn direct_material_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vertex_shader: &wgpu::ShaderModule,
    fragment_shader: &wgpu::ShaderModule,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Prism direct SDR material pipeline"),
        layout: Some(layout),
        vertex: vertex_state(vertex_shader),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: fragment_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                None,
            ],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn composite_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vertex_shader: &wgpu::ShaderModule,
    fragment_shader: &wgpu::ShaderModule,
    output_format: wgpu::TextureFormat,
    label: &'static str,
    hdr: bool,
    material: bool,
) -> wgpu::RenderPipeline {
    let constants = [
        ("hdr_enabled", u32::from(hdr) as f64),
        ("material_enabled", u32::from(material) as f64),
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: vertex_state(vertex_shader),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: fragment_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants,
                ..Default::default()
            },
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn render_texture(
    device: &wgpu::Device,
    label: &'static str,
    size: RenderTargetSize,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: INTERNAL_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn clear_pair(
    encoder: &mut wgpu::CommandEncoder,
    label: &'static str,
    base: &wgpu::TextureView,
    gain: Option<&wgpu::TextureView>,
) {
    let color_attachments = [
        Some(wgpu::RenderPassColorAttachment {
            view: base,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        }),
        gain.map(|view| wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        }),
    ];
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &color_attachments,
        ..Default::default()
    });
}

#[cfg(test)]
mod tests {
    use super::specialized_optical_shader;

    fn hdr_function(shader: &str) -> &str {
        let start = shader.find("fn encodeHdrGain(").unwrap();
        let end = start + shader[start..].find("\n}\n").unwrap() + 3;
        &shader[start..end]
    }

    #[test]
    fn specializes_each_optical_pass_before_pipeline_compilation() {
        let light = specialized_optical_shader(0, false);
        let material = specialized_optical_shader(1, false);
        let light_hdr = specialized_optical_shader(0, true);

        assert!(!light.contains("global.uRenderLayer"));
        assert!(!material.contains("global.uRenderLayer"));
        assert_ne!(light, material);
        assert!(hdr_function(&light).contains("return vec4(0f);"));
        assert!(!hdr_function(&light_hdr).contains("return vec4(0f);"));
    }
}
