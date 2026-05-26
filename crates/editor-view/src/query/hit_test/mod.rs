mod hit;

#[cfg(test)]
mod tests;

pub(crate) use hit::{HitTarget, HitTester, box_path_at, rect_distance_sq};
