//! Platform-neutral optical calculations used by every Prism renderer host.

use glam::{DMat3, DQuat, DVec3};

pub const MAX_LIGHTS: usize = 3;
pub const MAX_SOURCE_SAMPLES: usize = 5;
pub const SPECTRAL_ANCHOR_COUNT: usize = 7;

const PI: f64 = std::f64::consts::PI;
const OPTICAL_PATH_CENTER_SAMPLE: usize = MAX_SOURCE_SAMPLES;
const OPTICAL_LIGHT_PATH_COLUMNS: usize = MAX_LIGHTS * MAX_SOURCE_SAMPLES * SPECTRAL_ANCHOR_COUNT;
const OPTICAL_MATERIAL_PATH_COLUMNS: usize = MAX_LIGHTS * SPECTRAL_ANCHOR_COUNT;
const OPTICAL_LIGHT_PATH_ROWS: usize = 3;
const OPTICAL_MATERIAL_PATH_ROWS: usize = 6;
const OPTICAL_MATERIAL_PATH_OFFSET: usize = OPTICAL_LIGHT_PATH_COLUMNS * OPTICAL_LIGHT_PATH_ROWS;

pub const OPTICAL_PATH_BUFFER_FLOATS: usize =
    (OPTICAL_MATERIAL_PATH_OFFSET + OPTICAL_MATERIAL_PATH_COLUMNS * OPTICAL_MATERIAL_PATH_ROWS) * 4;

pub const BASIC_PRISM_PLANES: [f32; 20] = [
    0.0,
    -1.0,
    0.0,
    0.49,
    0.866_025_4,
    0.5,
    0.0,
    0.49,
    -0.866_025_4,
    0.5,
    0.0,
    0.49,
    0.0,
    0.0,
    1.0,
    0.58,
    0.0,
    0.0,
    -1.0,
    0.58,
];

#[derive(Clone, Copy, Debug)]
pub struct OpticalFrame<'a> {
    pub planes: &'a [f32],
    pub plane_count: usize,
    pub transition_scale: f64,
    pub prism_scale: f64,
    pub bevel: f64,
    pub phase: f64,
    pub light_phase: f64,
    pub depth_transition: f64,
    pub perspective_transition: f64,
    pub object_pose_quaternion: Option<[f64; 4]>,
    pub light_count: usize,
    pub light_radius: f64,
    pub source_size: f64,
    pub source_divergence: f64,
    pub source_sample_count: usize,
    pub ior: f64,
    pub dispersion: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Intersection {
    near: f64,
    far: f64,
    near_plane: usize,
    far_plane: usize,
}

pub struct OpticalPathLookup {
    data: Box<[f32; OPTICAL_PATH_BUFFER_FLOATS]>,
    source_positions: [DVec3; MAX_LIGHTS * (MAX_SOURCE_SAMPLES + 1)],
    source_targets: [DVec3; MAX_LIGHTS * (MAX_SOURCE_SAMPLES + 1)],
}

impl Default for OpticalPathLookup {
    fn default() -> Self {
        Self {
            data: Box::new([0.0; OPTICAL_PATH_BUFFER_FLOATS]),
            source_positions: [DVec3::ZERO; MAX_LIGHTS * (MAX_SOURCE_SAMPLES + 1)],
            source_targets: [DVec3::ZERO; MAX_LIGHTS * (MAX_SOURCE_SAMPLES + 1)],
        }
    }
}

pub const fn optical_light_path_texel_offset(
    light_index: usize,
    sample_index: usize,
    anchor_index: usize,
    row: usize,
) -> usize {
    let column =
        (light_index * MAX_SOURCE_SAMPLES + sample_index) * SPECTRAL_ANCHOR_COUNT + anchor_index;
    (row * OPTICAL_LIGHT_PATH_COLUMNS + column) * 4
}

pub const fn optical_material_path_texel_offset(
    light_index: usize,
    anchor_index: usize,
    row: usize,
) -> usize {
    let column = light_index * SPECTRAL_ANCHOR_COUNT + anchor_index;
    (OPTICAL_MATERIAL_PATH_OFFSET + row * OPTICAL_MATERIAL_PATH_COLUMNS + column) * 4
}

impl OpticalPathLookup {
    pub fn update(&mut self, frame: &OpticalFrame<'_>) -> &[f32] {
        self.data.fill(0.0);
        let object_to_world = frame
            .object_pose_quaternion
            .and_then(|[x, y, z, w]| {
                let quaternion = DQuat::from_xyzw(x, y, z, w);
                (quaternion.is_finite() && quaternion.length_squared() > f64::EPSILON)
                    .then(|| quaternion.normalize())
            })
            .map(DMat3::from_quat)
            .unwrap_or_else(|| {
                let rotation_x = DMat3::from_rotation_x(-0.30 * frame.depth_transition);
                let rotation_y = DMat3::from_rotation_y(frame.phase * PI * 2.0);
                let rotation_z = DMat3::from_rotation_z(0.10 * frame.perspective_transition);
                rotation_y * rotation_x * rotation_z
            });
        let world_to_object = object_to_world.transpose();

        self.prepare_sources(frame, object_to_world);
        let light_count = frame.light_count.clamp(1, MAX_LIGHTS);
        let source_sample_count = frame.source_sample_count.clamp(1, MAX_SOURCE_SAMPLES);
        for light_index in 0..light_count {
            for sample_index in 0..source_sample_count {
                for anchor_index in 0..SPECTRAL_ANCHOR_COUNT {
                    self.trace_path(
                        frame,
                        object_to_world,
                        world_to_object,
                        light_index,
                        sample_index,
                        anchor_index,
                        ior_for_wavelength(
                            420.0 + anchor_index as f64 * 40.0,
                            frame.ior,
                            frame.dispersion,
                        ),
                    );
                }
            }
            for anchor_index in 0..SPECTRAL_ANCHOR_COUNT {
                self.trace_path(
                    frame,
                    object_to_world,
                    world_to_object,
                    light_index,
                    OPTICAL_PATH_CENTER_SAMPLE,
                    anchor_index,
                    ior_for_wavelength(
                        420.0 + anchor_index as f64 * 40.0,
                        frame.ior,
                        frame.dispersion,
                    ),
                );
            }
        }
        &self.data[..]
    }

    fn prepare_sources(&mut self, frame: &OpticalFrame<'_>, object_to_world: DMat3) {
        let light_count = frame.light_count.clamp(1, MAX_LIGHTS);
        let sample_count = frame.source_sample_count.clamp(1, MAX_SOURCE_SAMPLES);
        let relative_angle = (frame.light_phase - frame.phase) * PI * 2.0 - 0.73;
        for light_index in 0..light_count {
            let source_angle = relative_angle + light_index as f64 * 2.0 * PI / light_count as f64;
            let radius = frame.light_radius * frame.prism_scale;
            let light = object_to_world
                * DVec3::new(
                    source_angle.cos() * radius,
                    source_angle.sin() * radius,
                    0.0,
                );
            let forward = normalize_or_zero(-light);
            let reference = if forward.y.abs() < 0.92 {
                DVec3::Y
            } else {
                DVec3::X
            };
            let tangent = normalize_or_zero(forward.cross(reference));
            let bitangent = normalize_or_zero(tangent.cross(forward));

            for sample_index in 0..sample_count {
                let [offset_x, offset_y] = source_sample_offset(sample_index, sample_count);
                let plane_offset = tangent * offset_x + bitangent * offset_y;
                let index = light_index * (MAX_SOURCE_SAMPLES + 1) + sample_index;
                self.source_positions[index] =
                    light + plane_offset * frame.source_size * 0.58 * frame.prism_scale;
                self.source_targets[index] =
                    plane_offset * frame.source_divergence * 0.09 * frame.prism_scale;
            }
            let center_index = light_index * (MAX_SOURCE_SAMPLES + 1) + OPTICAL_PATH_CENTER_SAMPLE;
            self.source_positions[center_index] = light;
            self.source_targets[center_index] = DVec3::ZERO;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn trace_path(
        &mut self,
        frame: &OpticalFrame<'_>,
        object_to_world: DMat3,
        world_to_object: DMat3,
        light_index: usize,
        sample_index: usize,
        anchor_index: usize,
        ior: f64,
    ) {
        let source_index = light_index * (MAX_SOURCE_SAMPLES + 1) + sample_index;
        let origin = world_to_object * self.source_positions[source_index];
        let direction = normalize_or_zero(
            world_to_object
                * (self.source_targets[source_index] - self.source_positions[source_index]),
        );
        let Some(entry_hit) = intersect_prism(frame, origin, direction) else {
            return;
        };
        if entry_hit.near <= 0.0 {
            return;
        }

        let entry = origin + direction * entry_hit.near;
        let entry_normal = polished_normal(frame, entry, entry_hit.near_plane);
        let Some(mut inside_direction) = refract(direction, entry_normal, 1.0 / ior) else {
            return;
        };
        let mut inside_origin = entry + inside_direction * (0.0015 * frame.prism_scale);
        let mut exit = entry;
        let mut outgoing = DVec3::ZERO;
        let mut internal_length_a = 0.0;
        let mut internal_length_b = 0.0;
        let mut escaped = false;

        for bounce in 0..3 {
            let Some(exit_hit) = intersect_prism(frame, inside_origin, inside_direction) else {
                break;
            };
            exit = inside_origin + inside_direction * exit_hit.far;
            if bounce < 2 {
                let world_origin = object_to_world * inside_origin;
                let world_direction = normalize_or_zero(object_to_world * inside_direction);
                let origin_row = if bounce == 0 { 3 } else { 5 };
                let direction_row = if bounce == 0 { 4 } else { 6 };
                self.write_path_vector(
                    light_index,
                    sample_index,
                    anchor_index,
                    origin_row,
                    world_origin,
                    0.0,
                );
                self.write_path_vector(
                    light_index,
                    sample_index,
                    anchor_index,
                    direction_row,
                    world_direction,
                    0.0,
                );
                if bounce == 0 {
                    internal_length_a = exit_hit.far;
                } else {
                    internal_length_b = exit_hit.far;
                }
            }

            let exit_normal = polished_normal(frame, exit, exit_hit.far_plane);
            if let Some(candidate) = refract(inside_direction, -exit_normal, ior) {
                outgoing = normalize_or_zero(candidate);
                escaped = true;
                break;
            }
            inside_direction = normalize_or_zero(inside_direction.reflect(-exit_normal));
            inside_origin = exit + inside_direction * (0.0015 * frame.prism_scale);
        }
        if !escaped {
            return;
        }

        self.write_path_vector(
            light_index,
            sample_index,
            anchor_index,
            0,
            object_to_world * entry,
            1.0,
        );
        self.write_path_vector(
            light_index,
            sample_index,
            anchor_index,
            1,
            object_to_world * exit,
            internal_length_a,
        );
        self.write_path_vector(
            light_index,
            sample_index,
            anchor_index,
            2,
            normalize_or_zero(object_to_world * outgoing),
            internal_length_b,
        );
        if sample_index == OPTICAL_PATH_CENTER_SAMPLE {
            self.data[optical_material_path_texel_offset(light_index, anchor_index, 2) + 3] = 1.0;
        }
    }

    fn write_path_vector(
        &mut self,
        light_index: usize,
        sample_index: usize,
        anchor_index: usize,
        row: usize,
        value: DVec3,
        w: f64,
    ) {
        let offset = if sample_index == OPTICAL_PATH_CENTER_SAMPLE {
            if row == 0 {
                return;
            }
            optical_material_path_texel_offset(light_index, anchor_index, row - 1)
        } else {
            if row >= OPTICAL_LIGHT_PATH_ROWS {
                return;
            }
            optical_light_path_texel_offset(light_index, sample_index, anchor_index, row)
        };
        self.data[offset] = value.x as f32;
        self.data[offset + 1] = value.y as f32;
        self.data[offset + 2] = value.z as f32;
        self.data[offset + 3] = w as f32;
    }
}

fn intersect_prism(
    frame: &OpticalFrame<'_>,
    origin: DVec3,
    direction: DVec3,
) -> Option<Intersection> {
    let mut hit = Intersection {
        near: -1e6,
        far: 1e6,
        ..Intersection::default()
    };
    for index in 0..frame.plane_count.min(frame.planes.len() / 4) {
        let offset = index * 4;
        let normal = DVec3::new(
            frame.planes[offset] as f64,
            frame.planes[offset + 1] as f64,
            frame.planes[offset + 2] as f64,
        );
        let constant = frame.planes[offset + 3] as f64 * frame.transition_scale;
        let denominator = normal.dot(direction);
        let numerator = constant - normal.dot(origin);
        if denominator.abs() < 1e-6 {
            if numerator < 0.0 {
                return None;
            }
            continue;
        }
        let distance = numerator / denominator;
        if denominator < 0.0 {
            if distance > hit.near {
                hit.near = distance;
                hit.near_plane = index;
            }
        } else if distance < hit.far {
            hit.far = distance;
            hit.far_plane = index;
        }
        if hit.near > hit.far {
            return None;
        }
    }
    (hit.far > hit.near.max(0.0)).then_some(hit)
}

fn polished_normal(frame: &OpticalFrame<'_>, point: DVec3, fallback_plane: usize) -> DVec3 {
    let width = (frame.bevel * frame.transition_scale).max(0.003);
    let mut accumulated = DVec3::ZERO;
    let mut total_weight = 0.0;
    for index in 0..frame.plane_count.min(frame.planes.len() / 4) {
        let offset = index * 4;
        let normal = DVec3::new(
            frame.planes[offset] as f64,
            frame.planes[offset + 1] as f64,
            frame.planes[offset + 2] as f64,
        );
        let constant = frame.planes[offset + 3] as f64 * frame.transition_scale;
        let distance = (constant - normal.dot(point)).max(0.0) / width;
        let weight = (-0.5 * distance * distance).exp();
        accumulated += normal * weight;
        total_weight += weight;
    }
    let offset = fallback_plane.min(frame.planes.len() / 4 - 1) * 4;
    let fallback = DVec3::new(
        frame.planes[offset] as f64,
        frame.planes[offset + 1] as f64,
        frame.planes[offset + 2] as f64,
    );
    let blend = smoothstep(0.02, 0.48, total_weight - 0.95);
    normalize_or_zero(fallback.lerp(normalize_or_zero(accumulated), blend))
}

fn refract(incident: DVec3, normal: DVec3, eta: f64) -> Option<DVec3> {
    let cosine = normal.dot(incident);
    let k = 1.0 - eta * eta * (1.0 - cosine * cosine);
    (k >= 0.0).then(|| eta * incident - (eta * cosine + k.sqrt()) * normal)
}

fn normalize_or_zero(value: DVec3) -> DVec3 {
    let length = value.length();
    if length > 1e-12 {
        value / length
    } else {
        DVec3::ZERO
    }
}

fn smoothstep(minimum: f64, maximum: f64, value: f64) -> f64 {
    let t = ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn source_sample_offset(index: usize, count: usize) -> [f64; 2] {
    if count <= 1 {
        return [0.0, 0.0];
    }
    if count <= 3 {
        return match index {
            0 => [-0.58, -0.34],
            1 => [0.58, -0.34],
            _ => [0.0, 0.68],
        };
    }
    match index {
        0 => [-0.62, -0.48],
        1 => [0.62, -0.48],
        2 => [-0.62, 0.48],
        3 => [0.62, 0.48],
        _ => [0.0, 0.0],
    }
}

fn ior_for_wavelength(wavelength_nm: f64, ior: f64, dispersion: f64) -> f64 {
    let wavelength = wavelength_nm * 0.001;
    let green_wavelength: f64 = 0.540;
    let cauchy_b: f64 = 0.016;
    let offset = cauchy_b * (1.0 / wavelength.powi(2) - 1.0 / green_wavelength.powi(2));
    (ior + offset * dispersion.max(0.0)).clamp(1.0, 2.8)
}

#[cfg(test)]
mod tests {
    use super::{BASIC_PRISM_PLANES, OpticalFrame, OpticalPathLookup};

    fn frame(
        phase: f64,
        light_phase: f64,
        object_pose_quaternion: Option<[f64; 4]>,
    ) -> OpticalFrame<'static> {
        OpticalFrame {
            planes: &BASIC_PRISM_PLANES,
            plane_count: 5,
            transition_scale: 1.0,
            prism_scale: 1.0,
            bevel: 0.04,
            phase,
            light_phase,
            depth_transition: 0.0,
            perspective_transition: 0.0,
            object_pose_quaternion,
            light_count: 1,
            light_radius: 1.25,
            source_size: 0.31,
            source_divergence: 0.62,
            source_sample_count: 1,
            ior: 1.6,
            dispersion: 1.25,
        }
    }

    #[test]
    fn pose_override_drives_the_same_optical_paths_as_its_equivalent_phase() {
        let quarter_turn = std::f64::consts::FRAC_PI_4;
        let mut phase_lookup = OpticalPathLookup::default();
        let phase_paths = phase_lookup.update(&frame(0.25, 0.2, None)).to_vec();
        let mut pose_lookup = OpticalPathLookup::default();
        let pose_paths = pose_lookup
            .update(&frame(
                0.0,
                -0.05,
                Some([0.0, quarter_turn.sin(), 0.0, quarter_turn.cos()]),
            ))
            .to_vec();

        let maximum_delta = phase_paths
            .iter()
            .zip(pose_paths)
            .map(|(phase, pose)| (phase - pose).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_delta < 0.000_01,
            "maximum optical-path delta was {maximum_delta}"
        );
    }
}
