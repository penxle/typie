const NUM_LUM_BITS: u32 = 3;
const NUM_TABLES: usize = 1 << NUM_LUM_BITS;
const TABLE_WIDTH: usize = 256;

pub(crate) struct MaskGamma {
    tables: Box<[u8; NUM_TABLES * TABLE_WIDTH]>,
}

pub(crate) struct PreBlend<'a> {
    table: Option<&'a [u8; TABLE_WIDTH]>,
}

impl MaskGamma {
    pub fn new(contrast: f32, device_gamma: f32) -> Self {
        let mut tables = Box::new([0u8; NUM_TABLES * TABLE_WIDTH]);

        for i in 0..NUM_TABLES {
            let lum = scale255(i as u8);
            build_correcting_lut(
                &mut tables[i * TABLE_WIDTH..(i + 1) * TABLE_WIDTH],
                lum,
                contrast,
                device_gamma,
            );
        }

        Self { tables }
    }

    pub fn pre_blend(&self, luminance: u8) -> PreBlend<'_> {
        if self.tables[TABLE_WIDTH - 1] == 0 && self.tables[0] == 0 {
            return PreBlend { table: None };
        }

        let index = (luminance >> (8 - NUM_LUM_BITS)) as usize;
        let start = index * TABLE_WIDTH;
        let slice = &self.tables[start..start + TABLE_WIDTH];
        PreBlend {
            table: Some(slice.try_into().unwrap()),
        }
    }
}

impl<'a> PreBlend<'a> {
    #[inline(always)]
    pub fn apply(&self, alpha: u8) -> u8 {
        match self.table {
            Some(t) => t[alpha as usize],
            None => alpha,
        }
    }
}

fn scale255(value: u8) -> u8 {
    let base = (value as u32) << (8 - NUM_LUM_BITS);
    let mut lum = base;
    let mut i = NUM_LUM_BITS;
    while i < 8 {
        lum |= base >> i;
        i += NUM_LUM_BITS;
    }
    lum as u8
}

fn srgb_to_luma(luminance: f32) -> f32 {
    if luminance <= 0.04045 {
        luminance / 12.92
    } else {
        ((luminance + 0.055) / 1.055).powf(2.4)
    }
}

fn luma_to_srgb(luma: f32) -> f32 {
    if luma <= 0.0031308 {
        luma * 12.92
    } else {
        1.055 * luma.powf(1.0 / 2.4) - 0.055
    }
}

fn to_luma(gamma: f32, luminance: f32) -> f32 {
    if gamma == 0.0 {
        srgb_to_luma(luminance)
    } else if gamma == 1.0 {
        luminance
    } else {
        luminance.powf(gamma)
    }
}

fn from_luma(gamma: f32, luma: f32) -> f32 {
    if gamma == 0.0 {
        luma_to_srgb(luma)
    } else if gamma == 1.0 {
        luma
    } else {
        luma.powf(1.0 / gamma)
    }
}

fn apply_contrast(srca: f32, contrast: f32) -> f32 {
    srca + ((1.0 - srca) * contrast * srca)
}

fn build_correcting_lut(table: &mut [u8], src_i: u8, contrast: f32, device_gamma: f32) {
    let src = src_i as f32 / 255.0;
    let lin_src = to_luma(device_gamma, src);

    let dst = 1.0 - src;
    let lin_dst = to_luma(device_gamma, dst);

    let adjusted_contrast = contrast * lin_dst;

    if (src - dst).abs() < (1.0 / 256.0) {
        for i in 0..256 {
            let raw_srca = i as f32 / 255.0;
            let srca = apply_contrast(raw_srca, adjusted_contrast);
            table[i] = (srca * 255.0 + 0.5).min(255.0) as u8;
        }
    } else {
        for i in 0..256 {
            let raw_srca = i as f32 / 255.0;
            let srca = apply_contrast(raw_srca, adjusted_contrast);
            let dsta = 1.0 - srca;

            let lin_out = lin_src * srca + dsta * lin_dst;
            let out = from_luma(device_gamma, lin_out.min(1.0));

            let result = (out - dst) / (src - dst);
            table[i] = (result * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
    }
}

pub(crate) fn compute_luminance(r: u8, g: u8, b: u8) -> u8 {
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;

    let r_lin = srgb_to_luma(r_f);
    let g_lin = srgb_to_luma(g_f);
    let b_lin = srgb_to_luma(b_f);

    let luma = r_lin * 0.2126 + g_lin * 0.7152 + b_lin * 0.0722;
    let lum = luma_to_srgb(luma);
    (lum * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}
