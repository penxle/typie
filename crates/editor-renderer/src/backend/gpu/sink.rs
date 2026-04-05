use editor_common::Rect;
use std::sync::Arc;

use super::device::GpuDevice;
use super::submitter::GpuSubmitter;
use crate::error::RendererError;
use crate::sink::RenderSink;
use crate::types::{Color, Image, Path, Stroke, Transform};

pub struct GpuSink {
    device: Arc<GpuDevice>,
    submitter: GpuSubmitter,
    scene: vello::Scene,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    width: u32,
    height: u32,
}

impl GpuSink {
    pub fn new(
        device: Arc<GpuDevice>,
        surface: wgpu::Surface<'static>,
    ) -> Result<Self, RendererError> {
        let caps = surface.get_capabilities(&device.adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);
        use wgpu::CompositeAlphaMode::*;
        let alpha_mode = if caps.alpha_modes.contains(&PreMultiplied)
            || device.adapter.get_info().backend == wgpu::Backend::BrowserWebGpu
        {
            // WebGPU reports [Opaque] but supports PreMultiplied in practice
            PreMultiplied
        } else {
            [PostMultiplied, Opaque]
                .into_iter()
                .find(|m| caps.alpha_modes.contains(m))
                .unwrap_or(caps.alpha_modes[0])
        };
        let submitter = GpuSubmitter::new(Arc::clone(&device))?;

        Ok(Self {
            device,
            submitter,
            scene: vello::Scene::new(),
            surface,
            format,
            alpha_mode,
            width: 0,
            height: 0,
        })
    }

    pub fn present(&mut self) -> Result<(), RendererError> {
        let scene = std::mem::replace(&mut self.scene, vello::Scene::new());

        let texture = self.surface.get_current_texture()?;
        let result = self.submitter.render_to_surface(
            &scene,
            &texture,
            self.format,
            self.width,
            self.height,
        );

        texture.present();

        result
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.surface.configure(
            &self.device.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: self.alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }
}

impl RenderSink for GpuSink {
    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        let brush = peniko::Brush::Solid(color.into());
        let r = kurbo::Rect::new(
            rect.x as f64,
            rect.y as f64,
            (rect.x + rect.width) as f64,
            (rect.y + rect.height) as f64,
        );
        self.scene
            .fill(peniko::Fill::NonZero, transform.into(), &brush, None, &r);
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        let brush = peniko::Brush::Solid(color.into());
        self.scene.fill(
            peniko::Fill::NonZero,
            transform.into(),
            &brush,
            None,
            &kurbo::BezPath::from(path),
        );
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        let brush = peniko::Brush::Solid(color.into());
        let kurbo_stroke = kurbo::Stroke::new(stroke.width as f64);
        self.scene.stroke(
            &kurbo_stroke,
            transform.into(),
            &brush,
            None,
            &kurbo::BezPath::from(path),
        );
    }

    fn draw_image(&mut self, image: &Image, _rect: Rect, transform: Transform) {
        if image.width == 0 || image.height == 0 {
            return;
        }

        let blob = peniko::Blob::new(Arc::new(image.data.clone()));
        let image_data = peniko::ImageData {
            data: blob,
            format: peniko::ImageFormat::Rgba8,
            alpha_type: peniko::ImageAlphaType::AlphaPremultiplied,
            width: image.width,
            height: image.height,
        };
        let image_brush = peniko::ImageBrush::new(image_data);

        self.scene.draw_image(&image_brush, transform.into());
    }
}
