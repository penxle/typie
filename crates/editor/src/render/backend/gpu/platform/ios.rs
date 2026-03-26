use std::ffi::c_void;
use std::ptr::NonNull;

// ── Core Foundation FFI ─────────────────────────────────────────────────────

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFNumberRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFAllocatorRef = *const c_void;
type IOSurfaceRef = *mut c_void;
type CFIndex = isize;

const K_CF_NUMBER_SINT32_TYPE: CFIndex = 3;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFAllocatorDefault: CFAllocatorRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFNumberCreate(
        allocator: CFAllocatorRef,
        the_type: CFIndex,
        value_ptr: *const c_void,
    ) -> CFNumberRef;
    fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        num_values: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFDictionaryRef;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
}

#[link(name = "IOSurface", kind = "framework")]
unsafe extern "C" {
    static kIOSurfaceWidth: CFStringRef;
    static kIOSurfaceHeight: CFStringRef;
    static kIOSurfaceBytesPerElement: CFStringRef;
    static kIOSurfacePixelFormat: CFStringRef;
    fn IOSurfaceCreate(properties: CFDictionaryRef) -> IOSurfaceRef;
    fn IOSurfaceGetID(surface: IOSurfaceRef) -> u32;
    fn IOSurfaceLock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> i32;
    fn IOSurfaceUnlock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> i32;
    fn IOSurfaceGetBaseAddress(surface: IOSurfaceRef) -> *mut c_void;
}

// ── IOSurfaceWrapper ────────────────────────────────────────────────────────

pub struct IOSurfaceWrapper {
    surface: NonNull<c_void>,
}

// SAFETY: IOSurface는 프로세스 전역 공유 메모리이며 lock/unlock으로 동기화한다.
unsafe impl Send for IOSurfaceWrapper {}
unsafe impl Sync for IOSurfaceWrapper {}

impl IOSurfaceWrapper {
    /// BGRA8 포맷 IOSurface를 생성한다.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let pixel_format: u32 = u32::from_be_bytes(*b"BGRA");
        let bytes_per_element: u32 = 4;

        unsafe {
            let cf_width = CFNumberCreate(
                kCFAllocatorDefault,
                K_CF_NUMBER_SINT32_TYPE,
                &width as *const u32 as *const c_void,
            );
            let cf_height = CFNumberCreate(
                kCFAllocatorDefault,
                K_CF_NUMBER_SINT32_TYPE,
                &height as *const u32 as *const c_void,
            );
            let cf_bpe = CFNumberCreate(
                kCFAllocatorDefault,
                K_CF_NUMBER_SINT32_TYPE,
                &bytes_per_element as *const u32 as *const c_void,
            );
            let cf_format = CFNumberCreate(
                kCFAllocatorDefault,
                K_CF_NUMBER_SINT32_TYPE,
                &pixel_format as *const u32 as *const c_void,
            );

            let keys: [CFTypeRef; 4] = [
                kIOSurfaceWidth,
                kIOSurfaceHeight,
                kIOSurfaceBytesPerElement,
                kIOSurfacePixelFormat,
            ];
            let values: [CFTypeRef; 4] = [
                cf_width as CFTypeRef,
                cf_height as CFTypeRef,
                cf_bpe as CFTypeRef,
                cf_format as CFTypeRef,
            ];

            let dict = CFDictionaryCreate(
                kCFAllocatorDefault,
                keys.as_ptr(),
                values.as_ptr(),
                4,
                &kCFTypeDictionaryKeyCallBacks as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const c_void,
            );

            let surface = IOSurfaceCreate(dict);

            CFRelease(dict as CFTypeRef);
            CFRelease(cf_width as CFTypeRef);
            CFRelease(cf_height as CFTypeRef);
            CFRelease(cf_bpe as CFTypeRef);
            CFRelease(cf_format as CFTypeRef);

            Some(Self {
                surface: NonNull::new(surface)?,
            })
        }
    }

    pub fn as_ptr(&self) -> IOSurfaceRef {
        self.surface.as_ptr()
    }

    /// IOSurfaceID를 반환한다. Swift에서 `IOSurfaceLookup(surfaceID)`로 조회.
    pub fn surface_id(&self) -> u32 {
        unsafe { IOSurfaceGetID(self.surface.as_ptr()) }
    }

    /// native_handle = IOSurfaceID as u64. 스펙 섹션 6.2 참조.
    pub fn native_handle(&self) -> u64 {
        self.surface_id() as u64
    }

    /// CPU 쓰기 lock. 반환 포인터로 tiny-skia PixmapMut를 생성할 수 있다.
    pub fn lock_for_cpu_write(&self) -> Option<*mut u8> {
        let result = unsafe { IOSurfaceLock(self.surface.as_ptr(), 0, std::ptr::null_mut()) };
        if result != 0 {
            return None;
        }
        let addr = unsafe { IOSurfaceGetBaseAddress(self.surface.as_ptr()) };
        if addr.is_null() {
            unsafe { IOSurfaceUnlock(self.surface.as_ptr(), 0, std::ptr::null_mut()) };
            None
        } else {
            Some(addr as *mut u8)
        }
    }

    pub fn unlock(&self) {
        unsafe {
            IOSurfaceUnlock(self.surface.as_ptr(), 0, std::ptr::null_mut());
        }
    }
}

impl Drop for IOSurfaceWrapper {
    fn drop(&mut self) {
        unsafe { CFRelease(self.surface.as_ptr() as CFTypeRef) }
    }
}

// ── GPU import ──────────────────────────────────────────────────────────────

/// IOSurface를 wgpu Texture로 임포트한다.
/// Metal: MTLDevice.makeTexture(descriptor:iosurface:plane:) → wgpu HAL → wgpu::Texture.
pub fn import_as_wgpu_texture(
    device: &wgpu::Device,
    surface: &IOSurfaceWrapper,
    width: u32,
    height: u32,
) -> Option<wgpu::Texture> {
    use metal::foreign_types::ForeignType;
    use objc::{msg_send, sel, sel_impl};

    // 1. HAL device에서 Metal texture 생성 (guard scope 제한)
    let hal_texture = {
        let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Metal>()? };
        let raw_device = hal_device.raw_device();

        let descriptor = metal::TextureDescriptor::new();
        descriptor.set_texture_type(metal::MTLTextureType::D2);
        descriptor.set_pixel_format(metal::MTLPixelFormat::RGBA8Unorm);
        descriptor.set_width(width as u64);
        descriptor.set_height(height as u64);
        descriptor
            .set_usage(metal::MTLTextureUsage::RenderTarget | metal::MTLTextureUsage::ShaderRead);
        descriptor.set_storage_mode(metal::MTLStorageMode::Shared);

        // newTextureWithDescriptor:iosurface:plane: — metal crate에 미포함, objc FFI 사용
        let device_ptr = raw_device.as_ptr() as *mut objc::runtime::Object;
        let desc_ptr = descriptor.as_ptr() as *mut objc::runtime::Object;
        let raw_tex: *mut objc::runtime::Object = unsafe {
            msg_send![device_ptr, newTextureWithDescriptor:desc_ptr iosurface:surface.as_ptr() plane:0usize]
        };
        if raw_tex.is_null() {
            return None;
        }

        unsafe {
            wgpu::hal::metal::Device::texture_from_raw(
                metal::Texture::from_ptr(raw_tex as *mut _),
                wgpu::TextureFormat::Rgba8Unorm,
                metal::MTLTextureType::D2,
                1,
                1,
                wgpu::hal::CopyExtent {
                    width,
                    height,
                    depth: 1,
                },
            )
        }
    }; // HAL guard dropped

    // 2. wgpu::Texture로 래핑
    let desc = wgpu::TextureDescriptor {
        label: Some("ios-page-texture"),
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
    };

    Some(unsafe { device.create_texture_from_hal::<wgpu::hal::api::Metal>(hal_texture, &desc) })
}
