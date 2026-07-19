use editor_renderer::damage::IRect;

pub fn clamp_dim_u16(v: u32) -> u16 {
    v.min(u32::from(u16::MAX)) as u16
}

pub const CPU_SURFACE_BYTE_BUDGET: u64 = 512 * 1024 * 1024;

// present_damage의 putImageData 임시 스트립 버퍼 예산 — sink 예산(CPU_SURFACE_BYTE_BUDGET)과
// 별개로 작게 잡아, sink와 스트립이 동시 생존해도 hard peak이 512+64MiB로 bounded되게 한다.
// (이전엔 두 용도가 같은 512MiB 상수를 공유해 이론상 peak이 ~2×512MiB까지 갈 수 있었다.)
pub const CPU_PRESENT_STRIP_BYTE_BUDGET: u64 = 64 * 1024 * 1024;

pub fn cpu_surface_within_budget(w: u32, h: u32) -> bool {
    (w as u64)
        .checked_mul(h as u64)
        .and_then(|px| px.checked_mul(4))
        .is_some_and(|bytes| bytes <= CPU_SURFACE_BYTE_BUDGET)
}

pub fn max_strip_rows(width: u32, budget: u64) -> u32 {
    let per_row = u64::from(width) * 4;
    if per_row == 0 {
        return u32::MAX;
    }
    (budget / per_row).min(u64::from(u32::MAX)) as u32
}

pub fn tile_ranges(height: i32, max_tile: i32) -> Vec<(i32, i32)> {
    if max_tile <= 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut y = 0;
    while y < height {
        let h = max_tile.min(height - y);
        out.push((y, h));
        y += h;
    }
    out
}

pub fn clamp_rect(r: IRect, full: IRect) -> IRect {
    IRect {
        x0: r.x0.max(full.x0),
        y0: r.y0.max(full.y0),
        x1: r.x1.min(full.x1),
        y1: r.y1.min(full.y1),
    }
}

// probe 전용 debug readback의 상한: oversized/대형 요청을 거른다.
pub const DEBUG_READ_BYTE_CAP: u64 = 4 * 1024 * 1024;

// debug_read_surface_pixels 입구의 순수 rect 산술: (x,y,w,h) 요청을 surface 경계로 clamp하고
// w/h<=0·빈 교차·상한 초과를 모두 None으로 접는다. Some일 때만 실제 GL/CPU 판독을 수행한다.
// checked 산술 — width*height*4 오버플로도 None.
pub fn clamp_read_rect(x: i32, y: i32, w: i32, h: i32, bounds: IRect) -> Option<IRect> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let r = clamp_rect(
        IRect {
            x0: x,
            y0: y,
            x1: x.saturating_add(w),
            y1: y.saturating_add(h),
        },
        bounds,
    );
    if r.width() <= 0 || r.height() <= 0 {
        return None;
    }
    let bytes = (r.width() as u64)
        .checked_mul(r.height() as u64)
        .and_then(|px| px.checked_mul(4))?;
    if bytes > DEBUG_READ_BYTE_CAP {
        return None;
    }
    Some(r)
}

pub const SCRATCH_BYTE_CAP: u64 = 64 * 1024 * 1024;

// rect가 스크래치 바이트 상한을 넘으면 세로 스트립으로 분할한다(순수 — 테스트 필수:
// 분할 합집합 == 원 rect, 각 스트립 ≤ cap).
pub fn split_scratch_strips(r: IRect, byte_cap: u64) -> Vec<IRect> {
    let row_bytes = (r.width().max(1) as u64) * 4;
    let max_rows = (byte_cap / row_bytes).max(1) as i32;
    let mut out = Vec::new();
    let mut y = r.y0;
    while y < r.y1 {
        let y1 = (y + max_rows).min(r.y1);
        out.push(IRect {
            x0: r.x0,
            y0: y,
            x1: r.x1,
            y1,
        });
        y = y1;
    }
    out
}

// apply_damage 스크래치의 목표 크기를 결정한다(순수). 공통 케이스(목표가 byte_cap 이내)는
// 차원별 grow-only(max(cw, strip_w), max(ch, strip_h))를 유지해 반복 damage에서 재할당을
// 피한다. 그러나 grow-only 목표가 byte_cap을 넘으면 정확히 strip 크기로 축소(reset)한다 —
// strip은 split_scratch_strips에 의해 항상 ≤byte_cap이므로, 이 규칙 아래 스크래치 총량은
// byte_cap을 절대 넘지 않는다(불변식). 즉 SCRATCH_BYTE_CAP은 "각 스트립"뿐 아니라 스크래치
// sink 자체의 상한이기도 하다 — 넓은 스트립 이후 좁고 긴 스트립이 와도 sink가
// max(width)×max(height)로 무한 성장하지 않는다.
pub fn scratch_target_dims(
    current: (u16, u16),
    strip_w: u16,
    strip_h: u16,
    byte_cap: u64,
) -> (u16, u16) {
    let grown_w = current.0.max(strip_w);
    let grown_h = current.1.max(strip_h);
    let grown_bytes = (grown_w as u64) * (grown_h as u64) * 4;
    if grown_bytes > byte_cap {
        (strip_w, strip_h)
    } else {
        (grown_w, grown_h)
    }
}

// damage rect r과 타일 [tile_y0, tile_y0+tile_h)의 교차를 업로드 파라미터로 변환.
pub fn tile_copy_params(r: IRect, tile_y0: i32, tile_h: i32) -> Option<TileCopy> {
    let ty0 = r.y0.max(tile_y0);
    let ty1 = r.y1.min(tile_y0 + tile_h);
    if ty0 >= ty1 {
        return None;
    }
    Some(TileCopy {
        skip_rows: ty0 - r.y0,
        dst_x: r.x0,
        dst_y: ty0 - tile_y0,
        width: r.width(),
        rows: ty1 - ty0,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct TileCopy {
    pub skip_rows: i32,
    pub dst_x: i32,
    pub dst_y: i32,
    pub width: i32,
    pub rows: i32,
}

// 타일 → 백버퍼 blit의 dst Y 구간(GL 좌표, 수직 플립 포함). src는 (0,0)-(w,tile_h).
pub fn blit_dst_y(surface_h: i32, tile_y0: i32, tile_h: i32) -> (i32, i32) {
    (surface_h - tile_y0, surface_h - tile_y0 - tile_h)
}

#[cfg(feature = "wasm-browser")]
mod surface {
    use std::cell::RefCell;

    use editor_renderer::backend::cpu::CpuSink;
    use editor_renderer::display_list::DisplayList;
    use editor_renderer::sink::RenderSink;
    use js_sys::{Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{
        HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlFramebuffer, WebGlTexture,
    };

    use super::{
        IRect, SCRATCH_BYTE_CAP, blit_dst_y, clamp_rect, scratch_target_dims, split_scratch_strips,
        tile_copy_params, tile_ranges,
    };

    thread_local! {
        static SCRATCH: RefCell<CpuSink> = RefCell::new(CpuSink::new(1, 1));
    }

    struct Tile {
        tex: WebGlTexture,
        fbo: WebGlFramebuffer,
        y0: i32,
        height: i32,
    }

    #[derive(Clone, Copy)]
    pub struct GlLimits {
        pub max_tex: i32,
        pub max_viewport: (i32, i32),
    }

    thread_local! {
        static GL_LIMITS: std::cell::RefCell<Option<GlLimits>> = const { std::cell::RefCell::new(None) };
    }

    // 1×1 OffscreenCanvas로 한계만 조회하고 즉시 loseContext로 처분한다 — 페이지 캔버스를
    // 오염시키지 않고 GL 배정 가능 여부를 사전 판정하기 위한 프로브(컨텍스트 예산 비점유).
    // 성공값만 캐시한다: 일시적 프로브 실패(quota·진단 패치)가 탭 수명 전체의 CPU 고정으로
    // 굳지 않도록, 실패 시에는 다음 attach에서 재시도한다.
    pub fn gl_limits() -> Option<GlLimits> {
        if let Some(limits) = GL_LIMITS.with(|cell| *cell.borrow()) {
            return Some(limits);
        }
        let probed = probe_gl_limits();
        if probed.is_some() {
            GL_LIMITS.with(|cell| *cell.borrow_mut() = probed);
        }
        probed
    }

    fn probe_gl_limits() -> Option<GlLimits> {
        let canvas = web_sys::OffscreenCanvas::new(1, 1).ok()?;
        let gl = canvas.get_context("webgl2").ok()??.dyn_into::<Gl>().ok()?;
        // 조회 본체를 클로저로 실행하고, 결과와 무관하게 반드시 loseContext로 처분한다 —
        // ? 조기 탈출로 프로브 컨텍스트가 GC 전까지 누적되는 것을 막는다.
        let result = (|| {
            let max_tex = gl.get_parameter(Gl::MAX_TEXTURE_SIZE).ok()?.as_f64()? as i32;
            let dims = gl.get_parameter(Gl::MAX_VIEWPORT_DIMS).ok()?;
            let dims: js_sys::Int32Array = dims.dyn_into().ok()?;
            Some(GlLimits {
                max_tex,
                max_viewport: (dims.get_index(0), dims.get_index(1)),
            })
        })();
        if let Some(ext) = gl.get_extension("WEBGL_lose_context").ok().flatten() {
            let _ = js_sys::Reflect::get(&ext, &JsValue::from_str("loseContext"))
                .ok()
                .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
                .map(|f| f.call0(&ext));
        }
        result
    }

    pub struct GlPageSurface {
        canvas: HtmlCanvasElement,
        gl: Gl,
        tiles: Vec<Tile>,
        width: u32,
        height: u32,
        scale_factor: f64,
        limits: GlLimits,
        dead: bool,
    }

    impl GlPageSurface {
        pub fn new(
            canvas: HtmlCanvasElement,
            width: f64,
            height: f64,
            scale_factor: f64,
        ) -> Option<Self> {
            let pw = (width * scale_factor).round() as u32;
            let ph = (height * scale_factor).round() as u32;
            // 사전 판정: 컨텍스트를 만들기 전에 한계를 확인해 캔버스 오염 없이 CPU로 보낸다.
            let limits = gl_limits()?;
            if pw as i32 > limits.max_tex
                || pw as i32 > limits.max_viewport.0
                || ph as i32 > limits.max_viewport.1
                || pw > u32::from(u16::MAX)
                || ph > u32::from(u16::MAX)
            {
                web_sys::console::warn_1(
                    &format!("[gl-surface] page {pw}x{ph} exceeds GL limits; assigning cpu").into(),
                );
                return None;
            }
            let opts = Object::new();
            for (key, value) in [
                ("alpha", true),
                ("premultipliedAlpha", true),
                ("antialias", false),
                ("depth", false),
                ("stencil", false),
                ("preserveDrawingBuffer", false),
            ] {
                Reflect::set(&opts, &JsValue::from_str(key), &JsValue::from_bool(value)).ok()?;
            }
            // 여기까지의 실패(None)는 캔버스에 컨텍스트가 없다 — same-canvas CPU 폴백 안전.
            let gl = canvas
                .get_context_with_context_options("webgl2", opts.as_ref())
                .ok()??
                .dyn_into::<Gl>()
                .ok()?;
            // 이 지점부터 캔버스는 webgl2로 영구 고정 — 실패는 None이 아니라 dead로 표현한다.
            canvas.set_width(pw);
            canvas.set_height(ph);
            let buffer_ok =
                gl.drawing_buffer_width() == pw as i32 && gl.drawing_buffer_height() == ph as i32;
            let tiles = if buffer_ok {
                Self::create_tiles(&gl, pw as i32, ph as i32, limits.max_tex).unwrap_or_default()
            } else {
                Vec::new()
            };
            let dead = tiles.is_empty();
            if dead {
                web_sys::console::warn_1(
                    &format!("[gl-surface] page {pw}x{ph} context created but init failed; dead")
                        .into(),
                );
            }
            Some(Self {
                canvas,
                gl,
                tiles,
                width: pw,
                height: ph,
                scale_factor,
                limits,
                dead,
            })
        }

        pub fn is_dead(&self) -> bool {
            self.dead
        }

        // 부분 실패 시 이미 만든 타일과 진행 중이던 texture/FBO를 명시적으로 delete하고 None을
        // 반환한다(리소스 누수 방지 — 실패 주입 시 delete 호출을 검증할 것).
        // 모든 조기 반환 지점에서 이미 만든 자원을 정리한다 — cleanup 누락이 곧 GPU 누수다.
        fn create_tiles(gl: &Gl, width: i32, height: i32, max_tex: i32) -> Option<Vec<Tile>> {
            fn cleanup(
                gl: &Gl,
                tiles: &[Tile],
                tex: Option<&WebGlTexture>,
                fbo: Option<&WebGlFramebuffer>,
            ) {
                if let Some(fbo) = fbo {
                    gl.delete_framebuffer(Some(fbo));
                }
                if let Some(tex) = tex {
                    gl.delete_texture(Some(tex));
                }
                for tile in tiles {
                    gl.delete_framebuffer(Some(&tile.fbo));
                    gl.delete_texture(Some(&tile.tex));
                }
            }
            let mut tiles = Vec::new();
            for (y0, th) in tile_ranges(height, max_tex) {
                let Some(tex) = gl.create_texture() else {
                    cleanup(gl, &tiles, None, None);
                    return None;
                };
                gl.bind_texture(Gl::TEXTURE_2D, Some(&tex));
                gl.tex_storage_2d(Gl::TEXTURE_2D, 1, Gl::RGBA8, width, th);
                let Some(fbo) = gl.create_framebuffer() else {
                    cleanup(gl, &tiles, Some(&tex), None);
                    return None;
                };
                gl.bind_framebuffer(Gl::READ_FRAMEBUFFER, Some(&fbo));
                gl.framebuffer_texture_2d(
                    Gl::READ_FRAMEBUFFER,
                    Gl::COLOR_ATTACHMENT0,
                    Gl::TEXTURE_2D,
                    Some(&tex),
                    0,
                );
                if gl.check_framebuffer_status(Gl::READ_FRAMEBUFFER) != Gl::FRAMEBUFFER_COMPLETE {
                    cleanup(gl, &tiles, Some(&tex), Some(&fbo));
                    return None;
                }
                tiles.push(Tile {
                    tex,
                    fbo,
                    y0,
                    height: th,
                });
            }
            if gl.get_error() != Gl::NO_ERROR {
                cleanup(gl, &tiles, None, None);
                return None;
            }
            Some(tiles)
        }

        pub fn scale_factor(&self) -> f64 {
            self.scale_factor
        }

        pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
            let pw = (width * scale_factor).round() as u32;
            let ph = (height * scale_factor).round() as u32;
            if self.width == pw && self.height == ph && self.scale_factor == scale_factor {
                return false;
            }
            self.drop_tiles();
            self.width = pw;
            self.height = ph;
            self.scale_factor = scale_factor;
            self.canvas.set_width(pw);
            self.canvas.set_height(ph);
            let within_limits = pw as i32 <= self.limits.max_tex
                && pw as i32 <= self.limits.max_viewport.0
                && ph as i32 <= self.limits.max_viewport.1
                && pw <= u32::from(u16::MAX)
                && ph <= u32::from(u16::MAX);
            let buffer_ok = self.gl.drawing_buffer_width() == pw as i32
                && self.gl.drawing_buffer_height() == ph as i32;
            self.tiles = if within_limits && buffer_ok {
                Self::create_tiles(&self.gl, pw as i32, ph as i32, self.limits.max_tex)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            self.dead = self.tiles.is_empty();
            if self.dead {
                web_sys::console::warn_1(
                    &format!("[gl-surface] resize to {pw}x{ph} failed; surface dead").into(),
                );
            }
            true
        }

        pub fn apply_damage(&mut self, dl: &DisplayList, damage: &[IRect]) -> bool {
            if self.dead || self.tiles.is_empty() || self.gl.is_context_lost() {
                return false;
            }
            let full = IRect {
                x0: 0,
                y0: 0,
                x1: self.width as i32,
                y1: self.height as i32,
            };
            let uploaded = SCRATCH.with(|s| {
                let mut scratch = s.borrow_mut();
                for &raw in damage {
                    let r = clamp_rect(raw, full);
                    if r.width() <= 0 || r.height() <= 0 {
                        continue;
                    }
                    // 스크래치 상한: full-page damage가 GL 한계(≤16384²≈1GiB)까지 자라며 CPU
                    // 예산을 우회하는 것을 막는다 — 상한 초과 rect는 세로 스트립으로 분할해
                    // 래스터·업로드한다(분할 누적 == full은 accumulate oracle 계열로 검증).
                    // 스크래치 sink 자체도 SCRATCH_BYTE_CAP을 절대 넘지 않는다: grow-only 목표가
                    // 캡을 넘으면 strip 크기로 축소한다(scratch_target_dims 불변식).
                    for strip in split_scratch_strips(r, SCRATCH_BYTE_CAP) {
                        let (sw, sh) = scratch.pixel_size();
                        let current = (sw as u16, sh as u16);
                        let target = scratch_target_dims(
                            current,
                            strip.width() as u16,
                            strip.height() as u16,
                            SCRATCH_BYTE_CAP,
                        );
                        if target != current && !scratch.try_resize(target.0, target.1) {
                            return false;
                        }
                        editor_renderer::diff::raster_rect(dl, strip, &mut scratch);
                        if !self.upload_rect(&scratch, strip) {
                            return false;
                        }
                    }
                }
                true
            });
            if !uploaded {
                return false;
            }
            self.blit_all()
        }

        // 리테인 타일 전체를 백버퍼로 blit + flush (픽셀만; sink/DL/damage 장부 불변).
        fn blit_all(&self) -> bool {
            let h = self.height as i32;
            self.gl.bind_framebuffer(Gl::DRAW_FRAMEBUFFER, None);
            for tile in &self.tiles {
                self.gl
                    .bind_framebuffer(Gl::READ_FRAMEBUFFER, Some(&tile.fbo));
                let (dst_y0, dst_y1) = blit_dst_y(h, tile.y0, tile.height);
                self.gl.blit_framebuffer(
                    0,
                    0,
                    self.width as i32,
                    tile.height,
                    0,
                    dst_y0,
                    self.width as i32,
                    dst_y1,
                    Gl::COLOR_BUFFER_BIT,
                    Gl::NEAREST,
                );
            }
            self.gl.flush();
            self.gl.get_error() == Gl::NO_ERROR && !self.gl.is_context_lost()
        }

        // visibility 복귀 재blit — 죽음/로스면 복원 없이 false(호출부가 full로 강등). sink/DL/damage
        // 를 건드리지 않아 spurious 호출에도 안전하다.
        pub fn refresh(&mut self) -> bool {
            if self.tiles.is_empty() || self.gl.is_context_lost() {
                return false;
            }
            self.blit_all()
        }

        pub fn device_bounds(&self) -> IRect {
            IRect {
                x0: 0,
                y0: 0,
                x1: self.width as i32,
                y1: self.height as i32,
            }
        }

        // 각 타일의 device y0 목록(seam 텔레메트리용). 단일 타일이면 [0].
        pub fn tile_y0s(&self) -> Vec<i32> {
            self.tiles.iter().map(|t| t.y0).collect()
        }

        // probe 전용 판독: retained texture(내부 oracle). r은 이미 surface 경계로 clamp된
        // device rect, out은 r.width()*r.height()*4 크기.
        pub fn read_texture_pixels(&self, r: IRect, out: &mut [u8]) -> bool {
            if self.dead || self.gl.is_context_lost() {
                return false;
            }
            let mut ok = true;
            for tile in &self.tiles {
                let Some(copy) = tile_copy_params(r, tile.y0, tile.height) else {
                    continue;
                };
                self.gl
                    .bind_framebuffer(Gl::READ_FRAMEBUFFER, Some(&tile.fbo));
                let offset = (copy.skip_rows as usize) * (r.width() as usize) * 4;
                ok &= self
                    .gl
                    .read_pixels_with_opt_u8_array(
                        copy.dst_x,
                        copy.dst_y,
                        copy.width,
                        copy.rows,
                        Gl::RGBA,
                        Gl::UNSIGNED_BYTE,
                        Some(
                            &mut out
                                [offset..offset + (copy.rows as usize) * (copy.width as usize) * 4],
                        ),
                    )
                    .is_ok();
            }
            ok && self.gl.get_error() == Gl::NO_ERROR
        }

        // present oracle: 신선한 full blit 직후 같은 태스크 안에서 default framebuffer를 읽어
        // blit 산술·타일 배치·빈 백버퍼까지 검증한다(pDB:false여도 같은 태스크 내 판독은 유효).
        pub fn read_present_pixels(&mut self, r: IRect, out: &mut [u8]) -> bool {
            if self.dead || self.gl.is_context_lost() || !self.blit_all() {
                return false;
            }
            self.gl.bind_framebuffer(Gl::READ_FRAMEBUFFER, None);
            let h = self.height as i32;
            let mut ok = true;
            // readPixels는 GL 좌표(하→상)이므로 행 단위로 뒤집어 device 행 순서로 담는다.
            for row in 0..r.height() {
                let device_y = r.y0 + row;
                let gl_y = h - 1 - device_y;
                let offset = (row as usize) * (r.width() as usize) * 4;
                ok &= self
                    .gl
                    .read_pixels_with_opt_u8_array(
                        r.x0,
                        gl_y,
                        r.width(),
                        1,
                        Gl::RGBA,
                        Gl::UNSIGNED_BYTE,
                        Some(&mut out[offset..offset + (r.width() as usize) * 4]),
                    )
                    .is_ok();
            }
            ok && self.gl.get_error() == Gl::NO_ERROR
        }

        fn upload_rect(&self, scratch: &CpuSink, r: IRect) -> bool {
            let gl = &self.gl;
            let (scratch_w, _) = scratch.pixel_size();
            gl.pixel_storei(Gl::UNPACK_ROW_LENGTH, scratch_w as i32);
            gl.pixel_storei(Gl::UNPACK_SKIP_PIXELS, 0);
            let mut ok = true;
            for tile in &self.tiles {
                let Some(copy) = tile_copy_params(r, tile.y0, tile.height) else {
                    continue;
                };
                gl.bind_texture(Gl::TEXTURE_2D, Some(&tile.tex));
                gl.pixel_storei(Gl::UNPACK_SKIP_ROWS, copy.skip_rows);
                let view = unsafe { js_sys::Uint8Array::view(scratch.pixels()) };
                ok &= gl
                    .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                        Gl::TEXTURE_2D,
                        0,
                        copy.dst_x,
                        copy.dst_y,
                        copy.width,
                        copy.rows,
                        Gl::RGBA,
                        Gl::UNSIGNED_BYTE,
                        Some(view.as_ref()),
                    )
                    .is_ok();
                if !ok {
                    break;
                }
            }
            gl.pixel_storei(Gl::UNPACK_ROW_LENGTH, 0);
            gl.pixel_storei(Gl::UNPACK_SKIP_ROWS, 0);
            ok
        }

        fn drop_tiles(&mut self) {
            for tile in self.tiles.drain(..) {
                self.gl.delete_framebuffer(Some(&tile.fbo));
                self.gl.delete_texture(Some(&tile.tex));
            }
        }
    }

    // Drop은 타일 자원만 정리한다 — 컨텍스트 자체의 처분(loseContext)은 캔버스 노드를 소유한
    // TS가 수행한다(복원 재attach처럼 같은 캔버스에 재획득하는 경로가 있어 Rust가 임의로
    // lose하면 안 된다).
    impl Drop for GlPageSurface {
        fn drop(&mut self) {
            self.drop_tiles();
        }
    }
}

#[cfg(feature = "wasm-browser")]
pub use surface::GlPageSurface;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_dim_u16_caps_at_boundary() {
        assert_eq!(clamp_dim_u16(65_535), 65_535);
        assert_eq!(clamp_dim_u16(65_536), 65_535);
        assert_eq!(clamp_dim_u16(0), 0);
    }

    #[test]
    fn cpu_surface_within_budget_checks_area_times_four_bytes() {
        assert!(cpu_surface_within_budget(134_217_728, 1));
        assert!(!cpu_surface_within_budget(134_217_729, 1));
    }

    #[test]
    fn cpu_surface_within_budget_rejects_overflowing_area() {
        assert!(!cpu_surface_within_budget(u32::MAX, u32::MAX));
    }

    #[test]
    fn max_strip_rows_bounds_row_count_by_budget() {
        assert_eq!(max_strip_rows(65_535, CPU_SURFACE_BYTE_BUDGET), 2048);
        assert_eq!(max_strip_rows(1, CPU_SURFACE_BYTE_BUDGET), 134_217_728);
        assert_eq!(max_strip_rows(0, CPU_SURFACE_BYTE_BUDGET), u32::MAX);
    }

    // present_damage가 실제로 쓰는 상수(CPU_PRESENT_STRIP_BYTE_BUDGET, 64MiB) 기준 경계값 —
    // sink 예산과 분리된 이후에도 스트립 행 수가 예산 이내로 bounded됨을 고정한다.
    #[test]
    fn max_strip_rows_bounds_row_count_by_present_strip_budget() {
        assert_eq!(max_strip_rows(65_535, CPU_PRESENT_STRIP_BYTE_BUDGET), 256);
        assert_eq!(max_strip_rows(1, CPU_PRESENT_STRIP_BYTE_BUDGET), 16_777_216);
        assert_eq!(max_strip_rows(0, CPU_PRESENT_STRIP_BYTE_BUDGET), u32::MAX);
    }

    #[test]
    fn tile_ranges_splits_by_max() {
        assert_eq!(tile_ranges(10, 4), vec![(0, 4), (4, 4), (8, 2)]);
        assert_eq!(tile_ranges(4, 4), vec![(0, 4)]);
        assert_eq!(tile_ranges(0, 4), vec![]);
        assert_eq!(tile_ranges(5, 0), vec![]);
    }

    #[test]
    fn clamp_rect_bounds_to_full() {
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 200,
        };
        let r = IRect {
            x0: -5,
            y0: 150,
            x1: 120,
            y1: 260,
        };
        assert_eq!(
            clamp_rect(r, full),
            IRect {
                x0: 0,
                y0: 150,
                x1: 100,
                y1: 200
            }
        );
    }

    #[test]
    fn split_scratch_strips_covers_full_rect_within_cap() {
        let r = IRect {
            x0: 5,
            y0: 0,
            x1: 105,
            y1: 25,
        };
        let byte_cap = 100 * 4 * 10; // 10 rows per strip at width 100
        let strips = split_scratch_strips(r, byte_cap);
        assert_eq!(strips.len(), 3);
        assert_eq!(strips.first().unwrap().y0, r.y0);
        assert_eq!(strips.last().unwrap().y1, r.y1);
        for w in strips.windows(2) {
            assert_eq!(w[0].y1, w[1].y0);
        }
        for s in &strips {
            assert_eq!(s.x0, r.x0);
            assert_eq!(s.x1, r.x1);
            let bytes = (s.width() as u64) * (s.height() as u64) * 4;
            assert!(bytes <= byte_cap);
        }
    }

    #[test]
    fn split_scratch_strips_default_cap_keeps_typical_rect_whole() {
        let r = IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 50,
        };
        assert_eq!(split_scratch_strips(r, SCRATCH_BYTE_CAP), vec![r]);
    }

    #[test]
    fn split_scratch_strips_tiny_cap_still_makes_progress() {
        let r = IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 3,
        };
        // byte_cap smaller than one row: max_rows floors to 0, then .max(1) forces progress.
        let strips = split_scratch_strips(r, 1);
        assert_eq!(
            strips,
            vec![
                IRect {
                    x0: 0,
                    y0: 0,
                    x1: 100,
                    y1: 1
                },
                IRect {
                    x0: 0,
                    y0: 1,
                    x1: 100,
                    y1: 2
                },
                IRect {
                    x0: 0,
                    y0: 2,
                    x1: 100,
                    y1: 3
                },
            ]
        );
    }

    #[test]
    fn scratch_target_dims_grows_within_cap() {
        // 캡 이내면 차원별 grow-only 유지 — 한 차원에서만 커지는 damage가 반복돼도
        // 이미 확보한 다른 차원의 크기를 줄이지 않는다(할당 안정성, 공통 케이스).
        let byte_cap = SCRATCH_BYTE_CAP;
        assert_eq!(scratch_target_dims((100, 50), 40, 80, byte_cap), (100, 80));
        assert_eq!(scratch_target_dims((1, 1), 100, 50, byte_cap), (100, 50));
    }

    #[test]
    fn scratch_target_dims_shrinks_to_strip_when_grow_exceeds_cap() {
        // grow-only 목표(max(cw,strip_w) x max(ch,strip_h))가 byte_cap을 넘으면
        // 정확히 strip 크기로 축소한다 — strip 자체는 split_scratch_strips가 이미
        // ≤byte_cap을 보장하므로 결과도 캡 이내다.
        let byte_cap = 1000;
        let current = (16000, 1); // 넓고 얇은 이전 스트립 잔재
        let (strip_w, strip_h) = (1, 100); // 좁고 긴 새 스트립
        let target = scratch_target_dims(current, strip_w, strip_h, byte_cap);
        assert_eq!(target, (strip_w, strip_h));
        assert!((target.0 as u64) * (target.1 as u64) * 4 <= byte_cap);
    }

    #[test]
    fn scratch_target_dims_repeated_wide_then_narrow_tall_stays_within_cap() {
        // 회귀 재현: 넓은 스트립 다음 좁고 긴 스트립이 번갈아 오는 시퀀스에서, 이전
        // grow-only(max(sw,strip_w) x max(sh,strip_h)) 로직은 두 극단을 모두 흡수해
        // 스크래치가 최대 max_tex x 페이지높이(≈1GiB)까지 자랄 수 있었다. 수정 후에는
        // 매 단계 스크래치 바이트 총량이 byte_cap을 절대 넘지 않아야 한다.
        let byte_cap = SCRATCH_BYTE_CAP;
        let mut current = (1u16, 1u16);
        let sequence: &[(u16, u16)] = &[
            (16000, 4),
            (1, 4000),
            (16000, 4),
            (1, 4000),
            (8000, 2),
            (2, 8000),
        ];
        for &(strip_w, strip_h) in sequence {
            current = scratch_target_dims(current, strip_w, strip_h, byte_cap);
            let bytes = (current.0 as u64) * (current.1 as u64) * 4;
            assert!(
                bytes <= byte_cap,
                "scratch grew past cap: {current:?} -> {bytes} bytes"
            );
        }
    }

    #[test]
    fn tile_copy_params_covers_tile_boundaries() {
        let r = IRect {
            x0: 10,
            y0: 90,
            x1: 50,
            y1: 210,
        };
        assert_eq!(
            tile_copy_params(r, 0, 100),
            Some(TileCopy {
                skip_rows: 0,
                dst_x: 10,
                dst_y: 90,
                width: 40,
                rows: 10
            })
        );
        assert_eq!(
            tile_copy_params(r, 100, 100),
            Some(TileCopy {
                skip_rows: 10,
                dst_x: 10,
                dst_y: 0,
                width: 40,
                rows: 100
            })
        );
        assert_eq!(
            tile_copy_params(r, 200, 100),
            Some(TileCopy {
                skip_rows: 110,
                dst_x: 10,
                dst_y: 0,
                width: 40,
                rows: 10
            })
        );
        assert_eq!(tile_copy_params(r, 300, 100), None);
    }

    #[test]
    fn blit_dst_y_flips_vertically() {
        // device rows [0,100) → GL dst (H, H-100); 마지막 타일 [200,250) → (H-200, H-250).
        assert_eq!(blit_dst_y(250, 0, 100), (250, 150));
        assert_eq!(blit_dst_y(250, 200, 50), (50, 0));
    }

    const READ_BOUNDS: IRect = IRect {
        x0: 0,
        y0: 0,
        x1: 100,
        y1: 200,
    };

    #[test]
    fn clamp_read_rect_clamps_to_bounds() {
        assert_eq!(
            clamp_read_rect(90, 190, 40, 40, READ_BOUNDS),
            Some(IRect {
                x0: 90,
                y0: 190,
                x1: 100,
                y1: 200
            })
        );
    }

    #[test]
    fn clamp_read_rect_keeps_interior_rect_unchanged() {
        assert_eq!(
            clamp_read_rect(10, 20, 2, 2, READ_BOUNDS),
            Some(IRect {
                x0: 10,
                y0: 20,
                x1: 12,
                y1: 22
            })
        );
    }

    #[test]
    fn clamp_read_rect_rejects_nonpositive_dims() {
        assert_eq!(clamp_read_rect(0, 0, 0, 4, READ_BOUNDS), None);
        assert_eq!(clamp_read_rect(0, 0, 4, 0, READ_BOUNDS), None);
        assert_eq!(clamp_read_rect(0, 0, -4, 4, READ_BOUNDS), None);
    }

    #[test]
    fn clamp_read_rect_rejects_fully_out_of_bounds() {
        // origin past the right/bottom edge → empty intersection → None.
        assert_eq!(clamp_read_rect(100, 0, 4, 4, READ_BOUNDS), None);
        assert_eq!(clamp_read_rect(0, 200, 4, 4, READ_BOUNDS), None);
        assert_eq!(clamp_read_rect(-10, 0, 4, 4, READ_BOUNDS), None);
    }

    #[test]
    fn clamp_read_rect_rejects_oversized_request() {
        // A rect whose clamped area exceeds DEBUG_READ_BYTE_CAP (4 MiB / 4 bytes = 1 Mpx).
        let big = IRect {
            x0: 0,
            y0: 0,
            x1: 4096,
            y1: 4096,
        };
        assert_eq!(clamp_read_rect(0, 0, 4096, 4096, big), None);
        // Exactly at the cap (1024*1024 px == 4 MiB) is allowed.
        let cap = IRect {
            x0: 0,
            y0: 0,
            x1: 1024,
            y1: 1024,
        };
        assert_eq!(clamp_read_rect(0, 0, 1024, 1024, cap), Some(cap));
    }

    #[test]
    fn clamp_read_rect_saturating_origin_does_not_overflow() {
        // x + w would overflow i32; saturating_add keeps it finite, then clamp bounds it.
        assert_eq!(
            clamp_read_rect(i32::MAX - 1, 0, 8, 4, READ_BOUNDS),
            None,
            "origin past the right edge collapses to empty"
        );
    }
}
