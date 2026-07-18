use std::cell::RefCell;

use editor_renderer::damage::IRect;
use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d, OffscreenCanvas, WebGl2RenderingContext as Gl, WebGlFramebuffer,
    WebGlTexture,
};

use editor_renderer::backend::cpu::unpremultiply;

use crate::present::{Backoff, sentinel_color, sentinel_sink_offset, split_strips};

struct Presenter {
    canvas: OffscreenCanvas,
    gl: Gl,
    tex: WebGlTexture,
    fbo: WebGlFramebuffer,
    tex_w: i32,
    tex_h: i32,
    max_tex: i32,
    _on_lost: Closure<dyn FnMut(web_sys::Event)>,
}

struct GlState {
    presenter: Option<Presenter>,
    backoff: Backoff,
    nonce: u32,
}

thread_local! {
    static STATE: RefCell<GlState> = const {
        RefCell::new(GlState {
            presenter: None,
            backoff: Backoff::new(),
            nonce: 0,
        })
    };
    static CANARY: RefCell<Option<Function>> = const { RefCell::new(None) };
}

pub fn set_canary(callback: Function) {
    CANARY.with(|c| *c.borrow_mut() = Some(callback));
}

impl Presenter {
    fn create() -> Option<Self> {
        let canvas = OffscreenCanvas::new(1, 1).ok()?;
        let opts = Object::new();
        Reflect::set(&opts, &JsValue::from_str("alpha"), &JsValue::TRUE).ok()?;
        Reflect::set(
            &opts,
            &JsValue::from_str("premultipliedAlpha"),
            &JsValue::TRUE,
        )
        .ok()?;
        Reflect::set(&opts, &JsValue::from_str("antialias"), &JsValue::FALSE).ok()?;
        let gl = canvas
            .get_context_with_context_options("webgl2", opts.as_ref())
            .ok()??
            .dyn_into::<Gl>()
            .ok()?;
        let tex = gl.create_texture()?;
        let fbo = gl.create_framebuffer()?;
        let max_tex = gl.get_parameter(Gl::MAX_TEXTURE_SIZE).ok()?.as_f64()? as i32;
        let on_lost = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            let canary = CANARY.with(|c| c.borrow().clone());
            if let Some(f) = canary {
                let _ = f.call0(&JsValue::NULL);
            }
        });
        canvas
            .add_event_listener_with_callback("webglcontextlost", on_lost.as_ref().unchecked_ref())
            .ok()?;
        Some(Self {
            canvas,
            gl,
            tex,
            fbo,
            tex_w: 0,
            tex_h: 0,
            max_tex,
            _on_lost: on_lost,
        })
    }

    fn ensure_capacity(&mut self, w: i32, h: i32) -> bool {
        if w <= self.tex_w && h <= self.tex_h {
            return true;
        }
        let nw = w.max(self.tex_w);
        let nh = h.max(self.tex_h);
        self.gl.delete_texture(Some(&self.tex));
        let Some(tex) = self.gl.create_texture() else {
            return false;
        };
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&tex));
        self.gl.tex_storage_2d(Gl::TEXTURE_2D, 1, Gl::RGBA8, nw, nh);
        self.gl
            .bind_framebuffer(Gl::READ_FRAMEBUFFER, Some(&self.fbo));
        self.gl.framebuffer_texture_2d(
            Gl::READ_FRAMEBUFFER,
            Gl::COLOR_ATTACHMENT0,
            Gl::TEXTURE_2D,
            Some(&tex),
            0,
        );
        self.tex = tex;
        self.tex_w = nw;
        self.tex_h = nh;
        if (self.canvas.width() as i32) < nw {
            self.canvas.set_width(nw as u32);
        }
        if (self.canvas.height() as i32) < nh {
            self.canvas.set_height(nh as u32);
        }
        if self.gl.check_framebuffer_status(Gl::READ_FRAMEBUFFER) != Gl::FRAMEBUFFER_COMPLETE {
            return false;
        }
        if self.gl.get_error() != Gl::NO_ERROR {
            return false;
        }
        true
    }

    fn blit(
        &mut self,
        pixels: &[u8],
        sink_width: i32,
        r: IRect,
        ctx: &CanvasRenderingContext2d,
        nonce: u32,
    ) -> bool {
        let (w, h) = (r.width(), r.height());
        if w <= 0 || h <= 0 {
            return true;
        }
        if !self.ensure_capacity(w, h) {
            return false;
        }
        if self.gl.drawing_buffer_width() != self.canvas.width() as i32
            || self.gl.drawing_buffer_height() != self.canvas.height() as i32
        {
            return false;
        }
        let gl = &self.gl;
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.tex));
        gl.pixel_storei(Gl::UNPACK_ROW_LENGTH, sink_width);
        gl.pixel_storei(Gl::UNPACK_SKIP_PIXELS, r.x0);
        gl.pixel_storei(Gl::UNPACK_SKIP_ROWS, r.y0);
        let uploaded = {
            let view = unsafe { js_sys::Uint8Array::view(pixels) };
            gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                Gl::TEXTURE_2D,
                0,
                0,
                0,
                w,
                h,
                Gl::RGBA,
                Gl::UNSIGNED_BYTE,
                Some(view.as_ref()),
            )
            .is_ok()
        };
        gl.pixel_storei(Gl::UNPACK_ROW_LENGTH, 0);
        gl.pixel_storei(Gl::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_storei(Gl::UNPACK_SKIP_ROWS, 0);
        if !uploaded {
            return false;
        }
        gl.bind_framebuffer(Gl::READ_FRAMEBUFFER, Some(&self.fbo));
        gl.bind_framebuffer(Gl::DRAW_FRAMEBUFFER, None);
        let ch = gl.drawing_buffer_height();
        gl.blit_framebuffer(
            0,
            0,
            w,
            h,
            0,
            ch,
            w,
            ch - h,
            Gl::COLOR_BUFFER_BIT,
            Gl::NEAREST,
        );
        let sentinel = sentinel_color(nonce);
        gl.enable(Gl::SCISSOR_TEST);
        gl.scissor(w - 1, ch - h, 1, 1);
        gl.clear_color(
            f32::from(sentinel[0]) / 255.0,
            f32::from(sentinel[1]) / 255.0,
            f32::from(sentinel[2]) / 255.0,
            1.0,
        );
        gl.clear(Gl::COLOR_BUFFER_BIT);
        gl.disable(Gl::SCISSOR_TEST);
        gl.flush();
        if gl.is_context_lost() {
            return false;
        }
        ctx.save();
        let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        ctx.set_global_alpha(1.0);
        let _ = ctx.set_global_composite_operation("source-over");
        ctx.clear_rect(r.x0 as f64, r.y0 as f64, w as f64, h as f64);
        let ok = ctx
            .draw_image_with_offscreen_canvas_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &self.canvas,
                0.0,
                0.0,
                w as f64,
                h as f64,
                r.x0 as f64,
                r.y0 as f64,
                w as f64,
                h as f64,
            )
            .is_ok();
        ctx.restore();
        if !ok {
            return false;
        }
        verify_and_restore_sentinel(pixels, sink_width, r, ctx, sentinel)
    }
}

fn verify_and_restore_sentinel(
    pixels: &[u8],
    sink_width: i32,
    r: IRect,
    ctx: &CanvasRenderingContext2d,
    sentinel: [u8; 4],
) -> bool {
    let sx = (r.x1 - 1) as f64;
    let sy = (r.y1 - 1) as f64;
    let Ok(read) = ctx.get_image_data(sx, sy, 1.0, 1.0) else {
        return false;
    };
    let data = read.data();
    if data.0.get(0..4) != Some(&sentinel[..]) {
        web_sys::console::warn_1(
            &format!(
                "[gl-present] sentinel mismatch at ({sx},{sy}): expected {sentinel:?}, got {:?}",
                data.0.get(0..4)
            )
            .into(),
        );
        return false;
    }
    let offset = sentinel_sink_offset(sink_width, r);
    let Some(src) = pixels.get(offset..offset + 4) else {
        return false;
    };
    let mut px = [src[0], src[1], src[2], src[3]];
    unpremultiply(&mut px);
    let Ok(image_data) =
        web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&px[..]), 1, 1)
    else {
        return false;
    };
    ctx.put_image_data(&image_data, sx, sy).is_ok()
}

impl Drop for Presenter {
    fn drop(&mut self) {
        let _ = self.canvas.remove_event_listener_with_callback(
            "webglcontextlost",
            self._on_lost.as_ref().unchecked_ref(),
        );
    }
}

pub fn begin() -> bool {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if !st.backoff.allow() {
            return false;
        }
        let alive = st
            .presenter
            .as_ref()
            .is_some_and(|p| !p.gl.is_context_lost());
        if !alive {
            st.presenter = Presenter::create();
            if st.presenter.is_none() {
                st.backoff.fail();
                return false;
            }
        }
        true
    })
}

pub fn present(pixels: &[u8], sink_width: i32, r: IRect, ctx: &CanvasRenderingContext2d) -> bool {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let st = &mut *st;
        let Some(p) = st.presenter.as_mut() else {
            return false;
        };
        let max_tex = p.max_tex;
        for strip in split_strips(r, max_tex) {
            st.nonce = st.nonce.wrapping_add(1);
            if !p.blit(pixels, sink_width, strip, ctx, st.nonce) {
                st.presenter = None;
                st.backoff.fail();
                return false;
            }
        }
        true
    })
}
