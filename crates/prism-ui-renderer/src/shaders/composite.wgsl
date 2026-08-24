struct CompositeParameters {
  hdr_headroom: f32,
  _padding0: f32,
  _padding1: f32,
  _padding2: f32,
};

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
};

@group(0) @binding(0) var light_base: texture_2d<f32>;
@group(0) @binding(1) var light_gain: texture_2d<f32>;
@group(0) @binding(2) var material_base: texture_2d<f32>;
@group(0) @binding(3) var material_gain: texture_2d<f32>;
@group(0) @binding(4) var linear_sampler: sampler;
@group(0) @binding(5) var<uniform> parameters: CompositeParameters;
override hdr_enabled: u32 = 1u;
override material_enabled: u32 = 1u;

fn decode_gain(encoded: vec3f) -> vec3f {
  return exp2(encoded * 2.0) - vec3f(1.0);
}

@fragment
fn main(input: VertexOutput) -> @location(0) vec4f {
  let light = textureSample(light_base, linear_sampler, input.uv);
  var material = vec4f(0.0);
  if material_enabled != 0u {
    material = textureSample(material_base, linear_sampler, input.uv);
  }
  let base = material + light * (1.0 - material.a);
  var final_radiance = base.rgb;
  if hdr_enabled != 0u {
    let light_excess = textureSample(light_gain, linear_sampler, input.uv);
    let material_excess = textureSample(material_gain, linear_sampler, input.uv);
    let gain_coverage = max(light_excess.a, material_excess.a);
    let physical_excess = decode_gain(material_excess.rgb)
      + decode_gain(light_excess.rgb) * (1.0 - material.a);
    let display_scale = clamp(parameters.hdr_headroom - 1.0, 0.0, 1.5) / 3.0;
    final_radiance += physical_excess * display_scale * gain_coverage;
  }
  return vec4f(min(final_radiance, vec3f(parameters.hdr_headroom * base.a)), base.a);
}
