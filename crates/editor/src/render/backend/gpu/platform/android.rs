use std::ptr::NonNull;

// ── AHardwareBuffer FFI ─────────────────────────────────────────────────────

#[repr(C)]
pub struct AHardwareBuffer {
    _private: [u8; 0],
}

#[repr(C)]
struct AHardwareBufferDesc {
    width: u32,
    height: u32,
    layers: u32,
    format: u32,
    usage: u64,
    stride: u32,
    rfu0: u32,
    rfu1: u64,
}

const FORMAT_R8G8B8A8_UNORM: u32 = 1;
const USAGE_CPU_READ_OFTEN: u64 = 3;
const USAGE_CPU_WRITE_OFTEN: u64 = 3 << 4;
const USAGE_GPU_SAMPLED_IMAGE: u64 = 1 << 8;
const USAGE_GPU_COLOR_OUTPUT: u64 = 1 << 9;

#[link(name = "android")]
unsafe extern "C" {
    fn AHardwareBuffer_allocate(
        desc: *const AHardwareBufferDesc,
        out_buffer: *mut *mut AHardwareBuffer,
    ) -> i32;
    fn AHardwareBuffer_release(buffer: *mut AHardwareBuffer);
    fn AHardwareBuffer_lock(
        buffer: *mut AHardwareBuffer,
        usage: u64,
        fence: i32,
        rect: *const std::ffi::c_void,
        out_addr: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn AHardwareBuffer_unlock(buffer: *mut AHardwareBuffer, fence: *mut i32) -> i32;
    fn AHardwareBuffer_toHardwareBuffer(
        env: *mut std::ffi::c_void,
        buffer: *mut AHardwareBuffer,
    ) -> *mut std::ffi::c_void;
}

// ── AHardwareBufferWrapper ──────────────────────────────────────────────────

pub struct AHardwareBufferWrapper {
    ptr: NonNull<AHardwareBuffer>,
}

// SAFETY: AHardwareBuffer는 프로세스 전역 핸들이며 NDK 문서상 스레드 안전하다.
unsafe impl Send for AHardwareBufferWrapper {}
unsafe impl Sync for AHardwareBufferWrapper {}

impl AHardwareBufferWrapper {
    /// RGBA8 포맷, GPU+CPU 양용 AHardwareBuffer를 생성한다.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let desc = AHardwareBufferDesc {
            width,
            height,
            layers: 1,
            format: FORMAT_R8G8B8A8_UNORM,
            usage: USAGE_GPU_COLOR_OUTPUT
                | USAGE_GPU_SAMPLED_IMAGE
                | USAGE_CPU_READ_OFTEN
                | USAGE_CPU_WRITE_OFTEN,
            stride: 0,
            rfu0: 0,
            rfu1: 0,
        };
        let mut buffer: *mut AHardwareBuffer = std::ptr::null_mut();
        let result = unsafe { AHardwareBuffer_allocate(&desc, &mut buffer) };
        if result != 0 {
            return None;
        }
        Some(Self {
            ptr: NonNull::new(buffer)?,
        })
    }

    pub fn as_ptr(&self) -> *mut AHardwareBuffer {
        self.ptr.as_ptr()
    }

    /// AHardwareBuffer 포인터를 u64로 반환한다. UniFFI native_handle로 사용.
    pub fn native_handle(&self) -> u64 {
        self.ptr.as_ptr() as u64
    }

    /// CPU 쓰기 lock. 반환 포인터로 tiny-skia PixmapMut를 생성할 수 있다.
    pub fn lock_for_cpu_write(&self) -> Option<*mut u8> {
        let mut addr: *mut std::ffi::c_void = std::ptr::null_mut();
        let result = unsafe {
            AHardwareBuffer_lock(
                self.ptr.as_ptr(),
                USAGE_CPU_WRITE_OFTEN,
                -1,
                std::ptr::null(),
                &mut addr,
            )
        };
        if result != 0 || addr.is_null() {
            None
        } else {
            Some(addr as *mut u8)
        }
    }

    pub fn unlock(&self) {
        unsafe {
            AHardwareBuffer_unlock(self.ptr.as_ptr(), std::ptr::null_mut());
        }
    }

    /// AHardwareBuffer → Java HardwareBuffer 변환 (JNI).
    /// `env`는 JNI 환경 포인터 (`*mut JNIEnv`).
    ///
    /// # Safety
    /// `env`는 유효한 JNI 환경이어야 한다.
    pub unsafe fn to_java_hardware_buffer(
        &self,
        env: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void {
        unsafe { AHardwareBuffer_toHardwareBuffer(env, self.ptr.as_ptr()) }
    }
}

impl Drop for AHardwareBufferWrapper {
    fn drop(&mut self) {
        unsafe { AHardwareBuffer_release(self.ptr.as_ptr()) }
    }
}

// ── GPU import ──────────────────────────────────────────────────────────────

/// AHardwareBuffer를 wgpu Texture로 임포트한다.
/// VK_ANDROID_external_memory_android_hardware_buffer 확장으로
/// Vulkan 외부 메모리 임포트 → wgpu HAL 래핑 → wgpu::Texture.
pub fn import_as_wgpu_texture(
    instance: &wgpu::Instance,
    device: &wgpu::Device,
    buffer: &AHardwareBufferWrapper,
    width: u32,
    height: u32,
) -> Option<wgpu::Texture> {
    use ash::vk;

    let hal_texture = {
        let hal_instance = unsafe { instance.as_hal::<wgpu::hal::api::Vulkan>()? };
        let ash_instance = hal_instance.shared_instance().raw_instance();
        let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>()? };
        let ash_device = hal_device.raw_device();

        // 1. AHardwareBuffer properties 조회
        let ahb_ext = ash::android::external_memory_android_hardware_buffer::Device::new(
            ash_instance,
            ash_device,
        );
        let mut ahb_props = vk::AndroidHardwareBufferPropertiesANDROID::default();
        unsafe {
            ahb_ext
                .get_android_hardware_buffer_properties(
                    buffer.as_ptr() as *const vk::AHardwareBuffer,
                    &mut ahb_props,
                )
                .ok()?;
        }

        // 2. VkImage 생성 (외부 메모리 연결)
        let mut ext_mem_info = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
            .push_next(&mut ext_mem_info);

        let image = unsafe { ash_device.create_image(&image_info, None).ok()? };

        // 3. AHardwareBuffer를 VkDeviceMemory로 임포트
        let mem_reqs = unsafe { ash_device.get_image_memory_requirements(image) };
        let memory_type_index =
            (ahb_props.memory_type_bits & mem_reqs.memory_type_bits).trailing_zeros();

        let mut import_info = vk::ImportAndroidHardwareBufferInfoANDROID::default()
            .buffer(buffer.as_ptr() as *mut vk::AHardwareBuffer);
        let mut dedicated_info = vk::MemoryDedicatedAllocateInfo::default().image(image);

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(memory_type_index)
            .push_next(&mut dedicated_info)
            .push_next(&mut import_info);

        let memory = match unsafe { ash_device.allocate_memory(&alloc_info, None) } {
            Ok(m) => m,
            Err(_) => {
                unsafe { ash_device.destroy_image(image, None) };
                return None;
            }
        };

        if unsafe { ash_device.bind_image_memory(image, memory, 0) }.is_err() {
            unsafe {
                ash_device.free_memory(memory, None);
                ash_device.destroy_image(image, None);
            }
            return None;
        }

        // 4. wgpu HAL texture로 래핑
        let hal_desc = wgpu::hal::TextureDescriptor {
            label: Some("android-page-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUses::COLOR_TARGET | wgpu::TextureUses::RESOURCE,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: vec![],
        };

        unsafe {
            hal_device.texture_from_raw(
                image,
                &hal_desc,
                None, // wgpu-hal이 VkImage 파괴 처리
                wgpu::hal::vulkan::TextureMemory::Dedicated(memory),
            )
        }
    }; // HAL guard scope 종료

    // 5. wgpu::Texture 생성
    let desc = wgpu::TextureDescriptor {
        label: Some("android-page-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };

    Some(unsafe { device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(hal_texture, &desc) })
}
