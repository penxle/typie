use std::mem::{align_of, offset_of, size_of};

use prism_ui_renderer::FrameUniforms;

#[test]
fn frame_uniforms_match_the_web_runtime_abi() {
    assert_eq!(align_of::<FrameUniforms>(), 16);
    assert_eq!(size_of::<FrameUniforms>(), 672);
    assert_eq!(offset_of!(FrameUniforms, prism_planes), 80);
    assert_eq!(offset_of!(FrameUniforms, prism_plane_count), 400);
    assert_eq!(offset_of!(FrameUniforms, transition_geometry), 480);
    assert_eq!(offset_of!(FrameUniforms, transition_prism_scale), 524);
    assert_eq!(offset_of!(FrameUniforms, spinner_morph), 528);
    assert_eq!(offset_of!(FrameUniforms, object_pose_quaternion), 576);
    assert_eq!(
        offset_of!(FrameUniforms, material_spectral_normalization),
        608
    );
    assert_eq!(offset_of!(FrameUniforms, environment_luminance), 624);
    assert_eq!(offset_of!(FrameUniforms, environment_light_mix), 628);
    assert_eq!(offset_of!(FrameUniforms, render_prism_scale), 632);
    assert_eq!(offset_of!(FrameUniforms, scaled_scattering_falloff), 636);
    assert_eq!(offset_of!(FrameUniforms, icon_edge_color), 640);
    assert_eq!(offset_of!(FrameUniforms, object_projection), 656);
}
