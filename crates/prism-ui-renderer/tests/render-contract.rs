use std::mem::{align_of, offset_of, size_of};

use prism_ui_renderer::FrameUniforms;

#[test]
fn frame_uniforms_match_the_web_runtime_abi() {
    assert_eq!(align_of::<FrameUniforms>(), 16);
    assert_eq!(size_of::<FrameUniforms>(), 656);
    assert_eq!(offset_of!(FrameUniforms, prism_planes), 80);
    assert_eq!(offset_of!(FrameUniforms, prism_plane_count), 400);
    assert_eq!(offset_of!(FrameUniforms, transition_geometry), 464);
    assert_eq!(offset_of!(FrameUniforms, transition_prism_scale), 508);
    assert_eq!(offset_of!(FrameUniforms, spinner_morph), 512);
    assert_eq!(offset_of!(FrameUniforms, object_pose_quaternion), 560);
    assert_eq!(
        offset_of!(FrameUniforms, material_spectral_normalization),
        592
    );
    assert_eq!(offset_of!(FrameUniforms, environment_luminance), 608);
    assert_eq!(offset_of!(FrameUniforms, environment_light_mix), 612);
    assert_eq!(offset_of!(FrameUniforms, render_prism_scale), 616);
    assert_eq!(offset_of!(FrameUniforms, scaled_scattering_falloff), 620);
    assert_eq!(offset_of!(FrameUniforms, icon_edge_color), 624);
    assert_eq!(offset_of!(FrameUniforms, object_projection), 640);
}
