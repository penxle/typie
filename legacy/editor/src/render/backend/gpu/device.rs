use crate::render::cache::PageSnapshot;
use crate::render::geometry::{LayoutRect, PixelRect};
use rustc_hash::FxHashMap;
use vello::{AaConfig, AaSupport, RenderParams, RendererOptions, Scene};
use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, Device, Queue, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
};

/// Vello는 straight alpha를 출력하지만 WebGPU surface의 CompositeAlphaMode는
/// premultiplied만 지원한다. BlendState로 blit 시 RGB × A를 수행한다.
fn create_premul_blitter(device: &Device, format: TextureFormat) -> wgpu::util::TextureBlitter {
    wgpu::util::TextureBlitterBuilder::new(device, format)
        .blend_state(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
        })
        .build()
}

/// GPU 페이지별 캐시: wgpu 텍스처 + 스냅샷
#[allow(dead_code)]
pub struct GpuPageCache {
    pub texture: Texture,
    pub snapshot: PageSnapshot,
    pub snapshot_initialized: bool,
    pub width: u32,
    pub height: u32,
}

#[allow(dead_code)]
pub struct GpuDevice {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Device,
    pub queue: Queue,
    pub renderer: vello::Renderer,
    pub page_cache: FxHashMap<usize, GpuPageCache>,
    surface_blitter: Option<(TextureFormat, wgpu::util::TextureBlitter)>,
}

impl GpuDevice {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok()?;
        let features = adapter.features();
        let limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("typie-gpu"),
                required_features: features
                    & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::CLEAR_TEXTURE),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .ok()?;

        let renderer = vello::Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                ..Default::default()
            },
        )
        .ok()?;

        Some(Self {
            instance,
            adapter,
            device,
            queue,
            renderer,
            page_cache: FxHashMap::default(),
            surface_blitter: None,
        })
    }

    /// Scene을 wgpu Texture에 렌더링한다.
    pub fn render_scene(
        &mut self,
        scene: &Scene,
        texture_view: &TextureView,
        width: u32,
        height: u32,
    ) -> Result<(), vello::Error> {
        self.renderer.render_to_texture(
            &self.device,
            &self.queue,
            scene,
            texture_view,
            &RenderParams {
                base_color: vello::peniko::color::palette::css::TRANSPARENT,
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        )
    }

    /// Scene을 wgpu Surface에 렌더링한다 (WASM OffscreenCanvas용).
    /// Vello는 STORAGE_BINDING 텍스처에만 렌더링할 수 있으므로,
    /// 중간 텍스처에 렌더 후 TextureBlitter로 surface에 복사한다.
    pub fn render_to_surface(
        &mut self,
        scene: &Scene,
        surface_texture: &wgpu::SurfaceTexture,
        surface_format: TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<(), vello::Error> {
        // 1. 중간 텍스처에 렌더
        let intermediate = create_texture(
            &self.device,
            width,
            height,
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        let intermediate_view = intermediate.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_scene(scene, &intermediate_view, width, height)?;

        // 2. premul blitter 확보 (캐시 활용)
        if self
            .surface_blitter
            .as_ref()
            .is_none_or(|(f, _)| *f != surface_format)
        {
            self.surface_blitter = Some((
                surface_format,
                create_premul_blitter(&self.device, surface_format),
            ));
        }

        // 3. straight alpha → premultiplied alpha 변환하며 surface에 복사
        let blitter = &self.surface_blitter.as_ref().unwrap().1;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("surface-blit"),
            });
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        blitter.copy(
            &self.device,
            &mut encoder,
            &intermediate_view,
            &surface_view,
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// 페이지의 dirty rect를 계산한다. `PageSnapshot::compute_dirty_rects` 공유 로직 사용.
    ///
    /// 반환값: (dirty_rects, should_full_repaint)
    /// - dirty_rects가 비어있으면 렌더 건너뜀
    /// - should_full_repaint이면 캐시 텍스처를 직접 교체
    #[allow(dead_code)]
    pub fn compute_dirty_rects(
        &mut self,
        page_idx: usize,
        next_snapshot: &PageSnapshot,
        canvas_width: f32,
        canvas_height: f32,
    ) -> (Vec<LayoutRect>, bool) {
        let prev = self
            .page_cache
            .get(&page_idx)
            .map(|c| (&c.snapshot, c.snapshot_initialized));
        PageSnapshot::compute_dirty_rects(prev, next_snapshot, canvas_width, canvas_height)
    }

    /// Scene을 dirty rect 기반으로 캐시 텍스처에 렌더링한다.
    ///
    /// - dirty_rects 비어있음 → 아무것도 안 함
    /// - full_repaint → Scene을 캐시 텍스처에 직접 렌더
    /// - 그 외 → Scene을 임시 텍스처에 렌더 후 dirty 영역만 캐시 텍스처에 copy
    #[allow(dead_code)]
    pub fn render_page(
        &mut self,
        page_idx: usize,
        scene: &Scene,
        dirty_rects: &[LayoutRect],
        full_repaint: bool,
        scale: f32,
        snapshot: PageSnapshot,
        width: u32,
        height: u32,
    ) {
        if dirty_rects.is_empty() {
            return;
        }

        // 캐시 엔트리 확보
        self.ensure_page_cache(page_idx, width, height);

        if full_repaint {
            // 캐시 텍스처에 직접 렌더
            let cache = self.page_cache.get(&page_idx).unwrap();
            let view = cache
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let _ = self.render_scene(scene, &view, width, height);
        } else {
            // 임시 텍스처에 렌더 후 dirty 영역만 copy
            let temp = create_texture(
                &self.device,
                width,
                height,
                TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            );
            let temp_view = temp.create_view(&wgpu::TextureViewDescriptor::default());
            if self.render_scene(scene, &temp_view, width, height).is_ok() {
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("dirty-blit"),
                        });

                let cache = self.page_cache.get(&page_idx).unwrap();
                for rect in dirty_rects {
                    let Some(pixel) = PixelRect::from_layout_rect(*rect, scale, width, height)
                    else {
                        continue;
                    };
                    encoder.copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &temp,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: pixel.x,
                                y: pixel.y,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: &cache.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: pixel.x,
                                y: pixel.y,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width: pixel.width,
                            height: pixel.height,
                            depth_or_array_layers: 1,
                        },
                    );
                }

                self.queue.submit(std::iter::once(encoder.finish()));
            }
        }

        // 스냅샷 업데이트
        let cache = self.page_cache.get_mut(&page_idx).unwrap();
        cache.snapshot = snapshot;
        cache.snapshot_initialized = true;
    }

    /// 페이지 캐시의 스냅샷을 업데이트한다.
    #[allow(dead_code)]
    pub fn update_page_snapshot(
        &mut self,
        page_idx: usize,
        snapshot: PageSnapshot,
        width: u32,
        height: u32,
    ) {
        self.ensure_page_cache(page_idx, width, height);
        let cache = self.page_cache.get_mut(&page_idx).unwrap();
        cache.snapshot = snapshot;
        cache.snapshot_initialized = true;
    }

    fn ensure_page_cache(&mut self, page_idx: usize, width: u32, height: u32) {
        let needs_new = match self.page_cache.get(&page_idx) {
            None => true,
            Some(c) => c.width != width || c.height != height,
        };
        if needs_new {
            let texture = create_texture(
                &self.device,
                width,
                height,
                TextureUsages::STORAGE_BINDING
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_SRC
                    | TextureUsages::COPY_DST,
            );
            self.page_cache.insert(
                page_idx,
                GpuPageCache {
                    texture,
                    snapshot: PageSnapshot::default(),
                    snapshot_initialized: false,
                    width,
                    height,
                },
            );
        }
    }

    #[allow(dead_code)]
    pub fn prune_page_cache(&mut self, valid_page_count: usize) {
        self.page_cache
            .retain(|page_idx, _| *page_idx < valid_page_count);
    }
}

fn create_texture(device: &Device, width: u32, height: u32, usage: TextureUsages) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("page-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    })
}
