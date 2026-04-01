use std::sync::Arc;

use super::device::GpuDevice;
use crate::RendererError;

pub struct GpuSubmitter {
    device: Arc<GpuDevice>,
    renderer: vello::Renderer,
    surface_blitter: Option<(wgpu::TextureFormat, wgpu::util::TextureBlitter)>,
}

impl GpuSubmitter {
    pub fn new(device: Arc<GpuDevice>) -> Result<Self, RendererError> {
        let renderer = vello::Renderer::new(
            &device.device,
            vello::RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::area_only(),
                ..Default::default()
            },
        )?;

        Ok(Self {
            device,
            renderer,
            surface_blitter: None,
        })
    }

    pub fn render_to_surface(
        &mut self,
        scene: &vello::Scene,
        surface_texture: &wgpu::SurfaceTexture,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        let intermediate = self.device.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let intermediate_view = intermediate.create_view(&wgpu::TextureViewDescriptor::default());

        self.renderer.render_to_texture(
            &self.device.device,
            &self.device.queue,
            scene,
            &intermediate_view,
            &vello::RenderParams {
                base_color: vello::peniko::color::palette::css::TRANSPARENT,
                width,
                height,
                antialiasing_method: vello::AaConfig::Area,
            },
        )?;

        if self
            .surface_blitter
            .as_ref()
            .is_none_or(|(f, _)| *f != surface_format)
        {
            self.surface_blitter = Some((
                surface_format,
                wgpu::util::TextureBlitterBuilder::new(&self.device.device, surface_format)
                    .blend_state(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                    })
                    .build(),
            ));
        }

        let blitter = &self.surface_blitter.as_ref().unwrap().1;

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        blitter.copy(
            &self.device.device,
            &mut encoder,
            &intermediate_view,
            &surface_view,
        );

        self.device.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
