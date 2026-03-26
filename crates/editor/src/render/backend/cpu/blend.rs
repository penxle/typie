// tiny-skia의 hot path인 blend를 low level로 구현한 것

pub(crate) fn blend_row_src_over(src: &[u8], dst: &mut [u8]) {
    debug_assert_eq!(src.len(), dst.len());
    debug_assert_eq!(src.len() % 4, 0);

    let mut i = 0usize;
    while i < src.len() {
        let src_alpha = src[i + 3];
        match src_alpha {
            0 => {}
            255 => {
                dst[i] = src[i];
                dst[i + 1] = src[i + 1];
                dst[i + 2] = src[i + 2];
                dst[i + 3] = src_alpha;
            }
            _ => {
                let inv_alpha = 255u16 - src_alpha as u16;
                let sr = src[i] as u16;
                let sg = src[i + 1] as u16;
                let sb = src[i + 2] as u16;
                let sa = src_alpha as u16;
                let dr = dst[i] as u16;
                let dg = dst[i + 1] as u16;
                let db = dst[i + 2] as u16;
                let da = dst[i + 3] as u16;
                dst[i] = (sr + ((dr * inv_alpha + 127) / 255)).min(255) as u8;
                dst[i + 1] = (sg + ((dg * inv_alpha + 127) / 255)).min(255) as u8;
                dst[i + 2] = (sb + ((db * inv_alpha + 127) / 255)).min(255) as u8;
                dst[i + 3] = (sa + ((da * inv_alpha + 127) / 255)).min(255) as u8;
            }
        }
        i += 4;
    }
}

pub(crate) fn build_const_src_over_lut(
    src: [u8; 4],
    lut_r: &mut [u8; 256],
    lut_g: &mut [u8; 256],
    lut_b: &mut [u8; 256],
    lut_a: &mut [u8; 256],
) {
    let inv_alpha = 255u16 - src[3] as u16;
    let sr = src[0] as u16;
    let sg = src[1] as u16;
    let sb = src[2] as u16;
    let sa = src[3] as u16;
    for d in 0u16..=255 {
        let idx = d as usize;
        lut_r[idx] = (sr + ((d * inv_alpha + 127) / 255)).min(255) as u8;
        lut_g[idx] = (sg + ((d * inv_alpha + 127) / 255)).min(255) as u8;
        lut_b[idx] = (sb + ((d * inv_alpha + 127) / 255)).min(255) as u8;
        lut_a[idx] = (sa + ((d * inv_alpha + 127) / 255)).min(255) as u8;
    }
}

pub(crate) fn blend_row_const_src_over_opaque(dst: &mut [u8], src: [u8; 4]) {
    debug_assert_eq!(dst.len() % 4, 0);
    let mut i = 0usize;
    while i < dst.len() {
        dst[i] = src[0];
        dst[i + 1] = src[1];
        dst[i + 2] = src[2];
        dst[i + 3] = src[3];
        i += 4;
    }
}

pub(crate) fn blend_row_const_src_over_lut(
    dst: &mut [u8],
    lut_r: &[u8; 256],
    lut_g: &[u8; 256],
    lut_b: &[u8; 256],
    lut_a: &[u8; 256],
) {
    debug_assert_eq!(dst.len() % 4, 0);
    let mut i = 0usize;
    while i < dst.len() {
        dst[i] = lut_r[dst[i] as usize];
        dst[i + 1] = lut_g[dst[i + 1] as usize];
        dst[i + 2] = lut_b[dst[i + 2] as usize];
        dst[i + 3] = lut_a[dst[i + 3] as usize];
        i += 4;
    }
}
