/// 마운트된 페이지의 버퍼 크기. Renderer 내부 저장 및 FFI 반환에 모두 사용한다.
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

#[cfg(target_os = "android")]
use crate::render::backend::gpu::platform::android::AHardwareBufferWrapper;
#[cfg(target_os = "ios")]
use crate::render::backend::gpu::platform::ios::IOSurfaceWrapper;

/// 플랫폼별 네이티브 텍스처 리소스.
/// CPU/GPU 백엔드 무관하게 동일한 버퍼 타입을 사용한다.
#[cfg(any(feature = "native", feature = "uniffi"))]
pub enum PlatformBuffer {
    #[cfg(target_os = "android")]
    Android { buffer: AHardwareBufferWrapper },
    #[cfg(target_os = "ios")]
    Ios { surface: IOSurfaceWrapper },
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    Desktop { pixel_data: Vec<u8> },
}
