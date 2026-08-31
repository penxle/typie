use bytemuck::pod_read_unaligned;
use prism_ui_renderer::{
    FrameUniforms, OpticalFrame, OpticalPathLookup, PrismRenderer, RenderFrame, RenderTargetSize,
    ScissorRectangle,
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
pub struct PrismWebRuntime {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

#[wasm_bindgen]
impl PrismWebRuntime {
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: HtmlCanvasElement) -> Result<PrismWebRuntime, JsValue> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(js_error)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Prism UI WebGPU device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    #[wasm_bindgen(js_name = createRenderer)]
    pub fn create_renderer(
        &self,
        canvas: HtmlCanvasElement,
        prefer_hdr: bool,
    ) -> Result<PrismWebRenderer, JsValue> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let surface = self
            .instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(js_error)?;
        let capabilities = surface.get_capabilities(&self.adapter);
        let hdr_supported = capabilities
            .formats
            .contains(&wgpu::TextureFormat::Rgba16Float);
        let hdr_surface = prefer_hdr && hdr_supported;
        let sdr_format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| *format != wgpu::TextureFormat::Rgba16Float)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| JsValue::from_str("No WebGPU surface format is available."))?;
        let format = if hdr_surface {
            wgpu::TextureFormat::Rgba16Float
        } else {
            sdr_format
        };
        let mut surface_config = surface
            .get_default_config(&self.adapter, width, height)
            .ok_or_else(|| {
                JsValue::from_str("The WebGPU adapter cannot present to this canvas.")
            })?;
        surface_config.format = format;
        surface_config.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
        surface_config.color_space = if hdr_surface {
            wgpu::SurfaceColorSpace::ExtendedSrgb
        } else {
            wgpu::SurfaceColorSpace::Auto
        };
        surface.configure(&self.device, &surface_config);

        Ok(PrismWebRenderer {
            surface,
            surface_config,
            renderer: PrismRenderer::new(self.device.clone(), self.queue.clone(), format),
            optical_paths: OpticalPathLookup::default(),
            hdr_supported,
            hdr_surface,
            sdr_format,
        })
    }
}

#[wasm_bindgen]
pub struct PrismWebRenderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: PrismRenderer,
    optical_paths: OpticalPathLookup,
    hdr_supported: bool,
    hdr_surface: bool,
    sdr_format: wgpu::TextureFormat,
}

#[wasm_bindgen]
impl PrismWebRenderer {
    #[wasm_bindgen(js_name = setHdrEnabled)]
    pub fn set_hdr_enabled(&mut self, enabled: bool) {
        let hdr_surface = enabled && self.hdr_supported;
        if hdr_surface == self.hdr_surface {
            return;
        }

        let format = if hdr_surface {
            wgpu::TextureFormat::Rgba16Float
        } else {
            self.sdr_format
        };
        self.surface_config.format = format;
        self.surface_config.color_space = if hdr_surface {
            wgpu::SurfaceColorSpace::ExtendedSrgb
        } else {
            wgpu::SurfaceColorSpace::Auto
        };
        let device = self.renderer.device().clone();
        let queue = self.renderer.queue().clone();
        self.surface.configure(&device, &self.surface_config);
        self.renderer = PrismRenderer::new(device, queue, format);
        self.hdr_surface = hdr_surface;
    }

    #[wasm_bindgen(getter, js_name = frameUniformByteLength)]
    pub fn frame_uniform_byte_length(&self) -> usize {
        std::mem::size_of::<FrameUniforms>()
    }

    #[wasm_bindgen(js_name = whenSubmittedWorkDone)]
    pub fn when_submitted_work_done(&self) -> js_sys::Promise {
        js_sys::Promise::new(&mut |resolve, _reject| {
            let resolve = resolve.clone();
            self.renderer.queue().on_submitted_work_done(move || {
                let _ = resolve.call0(&JsValue::UNDEFINED);
            });
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        frame_uniform_bytes: &[u8],
        optical_paths: &[f32],
        width: u32,
        height: u32,
        light_resolution_scale: f32,
        material_resolution_scale: f32,
        light_source_sample_count: i32,
        material_source_sample_count: i32,
        scissor_x: i32,
        scissor_y: i32,
        scissor_width: i32,
        scissor_height: i32,
        render_light: bool,
        hdr_headroom: f32,
    ) -> Result<(), JsValue> {
        if frame_uniform_bytes.len() != std::mem::size_of::<FrameUniforms>() {
            return Err(JsValue::from_str(
                "Invalid Prism frame-uniform byte length.",
            ));
        }
        let uniforms = pod_read_unaligned::<FrameUniforms>(frame_uniform_bytes);
        let material_scissor =
            (scissor_width > 0 && scissor_height > 0).then_some(ScissorRectangle {
                x: scissor_x.max(0) as u32,
                y: scissor_y.max(0) as u32,
                width: scissor_width as u32,
                height: scissor_height as u32,
            });
        render_to_surface(
            &self.surface,
            &mut self.surface_config,
            &mut self.renderer,
            self.hdr_surface,
            uniforms,
            optical_paths,
            width,
            height,
            light_resolution_scale,
            material_resolution_scale,
            light_source_sample_count,
            material_source_sample_count,
            material_scissor,
            render_light,
            hdr_headroom,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = renderComputed)]
    pub fn render_computed(
        &mut self,
        frame_uniform_bytes: &[u8],
        planes: &[f32],
        plane_count: usize,
        transition_scale: f64,
        prism_scale: f64,
        bevel: f64,
        phase: f64,
        light_phase: f64,
        depth_transition: f64,
        perspective_transition: f64,
        light_count: usize,
        light_radius: f64,
        source_size: f64,
        source_divergence: f64,
        source_sample_count: usize,
        ior: f64,
        dispersion: f64,
        width: u32,
        height: u32,
        light_resolution_scale: f32,
        material_resolution_scale: f32,
        light_source_sample_count: i32,
        material_source_sample_count: i32,
        scissor_x: i32,
        scissor_y: i32,
        scissor_width: i32,
        scissor_height: i32,
        render_light: bool,
        hdr_headroom: f32,
    ) -> Result<(), JsValue> {
        if frame_uniform_bytes.len() != std::mem::size_of::<FrameUniforms>() {
            return Err(JsValue::from_str(
                "Invalid Prism frame-uniform byte length.",
            ));
        }
        let uniforms = pod_read_unaligned::<FrameUniforms>(frame_uniform_bytes);
        let material_scissor =
            (scissor_width > 0 && scissor_height > 0).then_some(ScissorRectangle {
                x: scissor_x.max(0) as u32,
                y: scissor_y.max(0) as u32,
                width: scissor_width as u32,
                height: scissor_height as u32,
            });
        let optical_paths = self.optical_paths.update(&OpticalFrame {
            planes,
            plane_count,
            transition_scale,
            prism_scale,
            bevel,
            phase,
            light_phase,
            depth_transition,
            perspective_transition,
            object_pose_quaternion: (uniforms.object_pose_override > 0.5).then_some([
                uniforms.object_pose_quaternion[0] as f64,
                uniforms.object_pose_quaternion[1] as f64,
                uniforms.object_pose_quaternion[2] as f64,
                uniforms.object_pose_quaternion[3] as f64,
            ]),
            light_count,
            light_radius,
            source_size,
            source_divergence,
            source_sample_count,
            ior,
            dispersion,
        });
        render_to_surface(
            &self.surface,
            &mut self.surface_config,
            &mut self.renderer,
            self.hdr_surface,
            uniforms,
            optical_paths,
            width,
            height,
            light_resolution_scale,
            material_resolution_scale,
            light_source_sample_count,
            material_source_sample_count,
            material_scissor,
            render_light,
            hdr_headroom,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn render_to_surface(
    surface: &wgpu::Surface<'_>,
    surface_config: &mut wgpu::SurfaceConfiguration,
    renderer: &mut PrismRenderer,
    hdr_surface: bool,
    uniforms: FrameUniforms,
    optical_paths: &[f32],
    width: u32,
    height: u32,
    light_resolution_scale: f32,
    material_resolution_scale: f32,
    light_source_sample_count: i32,
    material_source_sample_count: i32,
    material_scissor: Option<ScissorRectangle>,
    render_light: bool,
    hdr_headroom: f32,
) -> Result<(), JsValue> {
    let width = width.max(1);
    let height = height.max(1);
    if surface_config.width != width || surface_config.height != height {
        surface_config.width = width;
        surface_config.height = height;
        surface.configure(renderer.device(), surface_config);
    }
    let mut reconfigured = false;
    let texture = loop {
        match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => break texture,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost
                if !reconfigured =>
            {
                surface.configure(renderer.device(), surface_config);
                reconfigured = true;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                return Err(JsValue::from_str(
                    "The WebGPU surface remained unavailable after reconfiguration.",
                ));
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(JsValue::from_str("WebGPU surface validation failed."));
            }
        }
    };
    let view = texture.texture.create_view(&Default::default());
    renderer
        .render(
            &view,
            &RenderFrame {
                uniforms,
                optical_paths,
                output_size: RenderTargetSize { width, height },
                light_resolution_scale,
                material_resolution_scale,
                light_source_sample_count,
                material_source_sample_count,
                material_scissor,
                render_light,
                hdr_headroom: if hdr_surface { hdr_headroom } else { 1.0 },
            },
        )
        .map_err(js_error)?;
    renderer.present(texture);
    Ok(())
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
