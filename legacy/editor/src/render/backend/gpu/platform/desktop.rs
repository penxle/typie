/// GPU 텍스처에서 pixel data를 readback한다.
/// wgpu Buffer(MAP_READ)로 복사 후 CPU에서 읽어 Vec<u8>로 반환한다.
#[allow(dead_code)]
pub fn readback_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let bytes_per_pixel = 4u32;
    let unpadded_row = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_row = (unpadded_row + align - 1) / align * align;
    let buffer_size = (padded_row * height) as u64;

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("desktop-readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("desktop-readback"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    let buffer_slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });

    if rx.recv().ok().and_then(|r| r.ok()).is_none() {
        return vec![0u8; (unpadded_row * height) as usize];
    }

    let data = buffer_slice.get_mapped_range();
    let unpadded = unpadded_row as usize;
    let padded = padded_row as usize;

    if padded == unpadded {
        data.to_vec()
    } else {
        let mut result = Vec::with_capacity(unpadded * height as usize);
        for row in 0..height as usize {
            result.extend_from_slice(&data[row * padded..row * padded + unpadded]);
        }
        result
    }
}
