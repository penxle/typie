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
}
