#version 300 es
// cspell:ignore henyey
// Canonical Prism material source, embedded and translated by prism-ui-renderer.
precision highp float;

uniform vec2 uResolution;
uniform float uPhase;
uniform float uLightPhase;
uniform int uLightCount;
uniform int uRenderLayer;
uniform float uIor;
uniform float uDispersion;
// Artistic glass clarity. Values above one reduce the neutral material veil,
// but must never create radiometric energy or brighten the backdrop.
uniform float uTransmission;
// Physical fraction of light that survives transmission through the prism.
uniform float uLightThroughput;
uniform float uMaterialOpacityScale;
uniform float uRoughness;
uniform float uFresnelStrength;
uniform float uSheenStrength;
uniform float uSheenWidth;
uniform float uSheenChroma;
uniform float uVisibility;
uniform float uBevel;
uniform vec4 uPrismPlanes[20];
uniform int uPrismPlaneCount;
uniform float uLightRadius;
uniform float uSourceSize;
uniform float uSourceDivergence;
uniform float uSourceHalo;
uniform int uSourceSampleCount;
uniform float uRayleighMix;
uniform float uCausticHalo;
uniform float uTurbulenceStrength;
uniform float uTurbulenceSpeed;
uniform float uOpticalTime;
uniform float uIncidentStrength;
uniform float uScatteringStrength;
uniform float uScatteringFalloff;
uniform float uSpectralFanReach;
uniform float uBeamWidth;
uniform vec4 uTransitionGeometry;
uniform vec4 uTransitionAppearance;
uniform float uIconSize;
uniform float uViewportScale;
uniform float uPrismScale;
uniform float uTransitionPrismScale;
uniform vec4 uSpinnerMorph;
uniform vec2 uSpinnerMaterialMorph;
uniform float uSpinnerMorphScale;
uniform float uSpinnerFrameSize;
uniform float uCssPixelRatio;
uniform float uSpinnerMorphMatchPhase;
uniform vec4 uObjectPoseQuaternion;
uniform float uObjectPoseOverride;
uniform float uSpinnerCreaseMorph;
uniform int uMaterialSpectralSampleCount;
uniform vec3 uMaterialSpectralNormalization;
uniform float uDarkMode;
uniform float uEnvironmentLuminance;
uniform float uEnvironmentLightMix;
uniform float uRenderPrismScale;
uniform float uScaledScatteringFalloff;
uniform vec4 uIconEdgeColor;
uniform vec4 uObjectProjection;

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outGain;

const float PI = 3.141592653589793;
const int MAX_PRISM_PLANES = 20;
const int MAX_LIGHTS = 3;
const int MAX_SOURCE_SAMPLES = 5;
const int SPECTRAL_ANCHOR_COUNT = 7;
const int OPTICAL_LIGHT_PATH_COLUMNS = MAX_LIGHTS * MAX_SOURCE_SAMPLES * SPECTRAL_ANCHOR_COUNT;
const int OPTICAL_MATERIAL_PATH_COLUMNS = MAX_LIGHTS * SPECTRAL_ANCHOR_COUNT;
const int OPTICAL_MATERIAL_PATH_OFFSET = OPTICAL_LIGHT_PATH_COLUMNS * 3;
const int OPTICAL_PATH_TEXEL_COUNT = OPTICAL_MATERIAL_PATH_OFFSET + OPTICAL_MATERIAL_PATH_COLUMNS * 6;
const float HDR_GAIN_MAX_RADIANCE = 4.0;
// The optical integrator uses scene-relative UI units rather than absolute
// nits. This one global exposure maps its reference white to SDR white while
// preserving the per-pixel radiance ordering and spectrum.
const float HDR_SCENE_EXPOSURE = 2.6;
// Keep radiance calibration tied to the optical medium. Diffuse air is a weak
// volumetric cue, while the directly dispersed fan and the illuminated glass
// volume carry the concentrated spectral energy that should reach HDR.
const float HDR_INCIDENT_AIR_GAIN = 0.18;
const float HDR_SPECTRAL_FAN_GAIN = 48.0;
const float HDR_GLASS_VOLUME_GAIN = 0.01;
const float HDR_FAN_AIR_RELATIVE_GAIN = 0.04;
const float DIRECT_SPECTRAL_COVERAGE_GAIN = 1.8;

float renderPrismScale();

vec4 encodeHdrGain(vec3 radiance, float coverage) {
  vec3 exposedRadiance = max(radiance, vec3(0.0)) * HDR_SCENE_EXPOSURE;
  // Crossing SDR white is one luminance event, not three independent channel
  // thresholds. Scale the whole radiance vector by the peak excess so a nearly
  // neutral highlight cannot become yellow merely because blue crossed the
  // threshold a little later than red and green.
  float peakRadiance = max(max(exposedRadiance.r, exposedRadiance.g), exposedRadiance.b);
  float excessScale = max(peakRadiance - 1.0, 0.0) / max(peakRadiance, 1e-5);
  vec3 excess = exposedRadiance * excessScale;
  vec3 encoded = log2(vec3(1.0) + min(excess, vec3(HDR_GAIN_MAX_RADIANCE - 1.0)))
    / log2(HDR_GAIN_MAX_RADIANCE);
  return vec4(encoded, clamp(coverage, 0.0, 1.0));
}

layout(std140) uniform OpticalPaths {
  vec4 uOpticalPathData[OPTICAL_PATH_TEXEL_COUNT];
};

float environmentLightMix() {
  return uEnvironmentLightMix;
}

float incidentAirDisplayScale() {
  // The air volume is only a directional cue. Bright environments used to
  // amplify it by up to 50x, making the atmosphere look emissive; retain the
  // existing dark-background floor and continuously cap the bright end.
  return mix(0.02, 0.14, environmentLightMix());
}

vec3 scatteringTint() {
  // Relative 1 / wavelength^4 weights at representative red, green and blue
  // wavelengths. Large aerosol particles remain close to neutral (Mie-like),
  // while the Rayleigh control introduces the familiar short-wave bias.
  vec3 mieTint = vec3(1.0);
  vec3 rayleighTint = vec3(0.21, 0.44, 1.0);
  return mix(mieTint, rayleighTint, clamp(uRayleighMix, 0.0, 1.0));
}

vec2 refractiveTurbulence(vec2 coordinate) {
  float time = uOpticalTime * uTurbulenceSpeed;
  vec2 position = coordinate * 3.1;
  float horizontal = sin(position.x * 1.7 + position.y * 1.1 + time * 1.13)
    + sin(position.x * 3.3 - position.y * 1.9 - time * 0.71) * 0.46;
  float vertical = sin(position.y * 1.5 - position.x * 0.9 - time * 0.89)
    + sin(position.y * 3.7 + position.x * 1.4 + time * 0.63) * 0.42;
  // One shared displacement field is applied before wavelength lookup, so the
  // entire spectrum wanders together instead of breaking into animated RGB.
  vec2 atmosphericDisplacement = vec2(horizontal, vertical)
    * uTurbulenceStrength
    * 0.0045
    * renderPrismScale();
  return atmosphericDisplacement;
}

vec2 sourceSampleOffset(int sampleIndex, int sampleCount) {
  if (sampleCount <= 1) return vec2(0.0);
  if (sampleCount <= 3) {
    if (sampleIndex == 0) return vec2(-0.58, -0.34);
    if (sampleIndex == 1) return vec2(0.58, -0.34);
    return vec2(0.0, 0.68);
  }
  if (sampleIndex == 0) return vec2(-0.62, -0.48);
  if (sampleIndex == 1) return vec2(0.62, -0.48);
  if (sampleIndex == 2) return vec2(-0.62, 0.48);
  if (sampleIndex == 3) return vec2(0.62, 0.48);
  return vec2(0.0);
}

void areaSourceRay(
  vec3 lightPosition,
  vec3 lightTarget,
  int sampleIndex,
  int sampleCount,
  out vec3 samplePosition,
  out vec3 sampleTarget
) {
  vec3 forward = normalize(lightTarget - lightPosition);
  vec3 referenceAxis = abs(forward.y) < 0.92 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
  vec3 tangent = normalize(cross(forward, referenceAxis));
  vec3 bitangent = normalize(cross(tangent, forward));
  vec2 offset = sourceSampleOffset(sampleIndex, sampleCount);
  vec3 planeOffset = tangent * offset.x + bitangent * offset.y;
  samplePosition = lightPosition + planeOffset * uSourceSize * 0.58 * renderPrismScale();
  sampleTarget = lightTarget + planeOffset * uSourceDivergence * 0.09 * renderPrismScale();
}

mat3 rotateX(float angle) {
  float c = cos(angle); float s = sin(angle);
  return mat3(
    1.0, 0.0, 0.0,
    0.0, c, s,
    0.0, -s, c
  );
}

mat3 rotateY(float angle) {
  float c = cos(angle); float s = sin(angle);
  return mat3(
    c, 0.0, -s,
    0.0, 1.0, 0.0,
    s, 0.0, c
  );
}

mat3 rotateZ(float angle) {
  float c = cos(angle); float s = sin(angle);
  return mat3(
    c, s, 0.0,
    -s, c, 0.0,
    0.0, 0.0, 1.0
  );
}

vec4 quaternionMultiply(vec4 left, vec4 right) {
  return vec4(
    left.w * right.xyz + right.w * left.xyz + cross(left.xyz, right.xyz),
    left.w * right.w - dot(left.xyz, right.xyz)
  );
}

vec4 quaternionFromAxisAngle(vec3 axis, float angle) {
  float halfAngle = angle * 0.5;
  return vec4(normalize(axis) * sin(halfAngle), cos(halfAngle));
}

vec4 quaternionConjugate(vec4 quaternion) {
  return vec4(-quaternion.xyz, quaternion.w);
}

vec4 normalizedQuaternionMix(vec4 start, vec4 end, float amount) {
  vec4 alignedEnd = dot(start, end) < 0.0 ? -end : end;
  return normalize(mix(start, alignedEnd, amount));
}

mat3 matrixFromQuaternion(vec4 quaternion) {
  vec4 q = normalize(quaternion);
  float xx = q.x * q.x;
  float yy = q.y * q.y;
  float zz = q.z * q.z;
  float xy = q.x * q.y;
  float xz = q.x * q.z;
  float yz = q.y * q.z;
  float xw = q.x * q.w;
  float yw = q.y * q.w;
  float zw = q.z * q.w;
  return mat3(
    1.0 - 2.0 * (yy + zz), 2.0 * (xy + zw), 2.0 * (xz - yw),
    2.0 * (xy - zw), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + xw),
    2.0 * (xz + yw), 2.0 * (yz - xw), 1.0 - 2.0 * (xx + yy)
  );
}

float renderPrismScale() {
  return uRenderPrismScale;
}

float transitionPrismScale() {
  return uTransitionPrismScale;
}

void prismPlane(int index, out vec3 normal, out float constantValue) {
  vec4 plane = uPrismPlanes[index];
  normal = plane.xyz;
  constantValue = plane.w * transitionPrismScale();
}

bool intersectPrism(
  vec3 origin,
  vec3 direction,
  out float nearDistance,
  out float farDistance,
  out vec3 nearNormal,
  out vec3 farNormal
) {
  nearDistance = -1e6;
  farDistance = 1e6;
  nearNormal = vec3(0.0);
  farNormal = vec3(0.0);

  for (int index = 0; index < MAX_PRISM_PLANES; index += 1) {
    if (index >= uPrismPlaneCount) break;
    vec3 planeNormal;
    float planeConstant;
    prismPlane(index, planeNormal, planeConstant);
    float denominator = dot(planeNormal, direction);
    float numerator = planeConstant - dot(planeNormal, origin);
    if (abs(denominator) < 1e-6) {
      if (numerator < 0.0) return false;
      continue;
    }
    float distanceValue = numerator / denominator;
    if (denominator < 0.0) {
      if (distanceValue > nearDistance) {
        nearDistance = distanceValue;
        nearNormal = planeNormal;
      }
    } else {
      if (distanceValue < farDistance) {
        farDistance = distanceValue;
        farNormal = planeNormal;
      }
    }
    if (nearDistance > farDistance) return false;
  }
  return farDistance > max(nearDistance, 0.0);
}

vec3 polishedNormal(vec3 point, vec3 fallbackNormal) {
  vec3 accumulated = vec3(0.0);
  float totalWeight = 0.0;
  float currentScale = transitionPrismScale();
  float width = max(uBevel * currentScale, 0.003);
  for (int index = 0; index < MAX_PRISM_PLANES; index += 1) {
    if (index >= uPrismPlaneCount) break;
    vec3 planeNormal;
    float planeConstant;
    prismPlane(index, planeNormal, planeConstant);
    float inwardDistance = max(planeConstant - dot(planeNormal, point), 0.0);
    float normalized = inwardDistance / width;
    float weight = exp(-0.5 * normalized * normalized);
    accumulated += planeNormal * weight;
    totalWeight += weight;
  }
  vec3 rounded = totalWeight > 1e-5 ? normalize(accumulated) : fallbackNormal;
  float blend = smoothstep(0.02, 0.48, totalWeight - 0.95);
  return normalize(mix(fallbackNormal, rounded, blend));
}

float edgeFactor(vec3 point) {
  float nearest = 1e6;
  float secondNearest = 1e6;
  for (int index = 0; index < MAX_PRISM_PLANES; index += 1) {
    if (index >= uPrismPlaneCount) break;
    vec3 planeNormal;
    float planeConstant;
    prismPlane(index, planeNormal, planeConstant);
    float distanceValue = abs(planeConstant - dot(planeNormal, point));
    if (distanceValue < nearest) {
      secondNearest = nearest;
      nearest = distanceValue;
    } else if (distanceValue < secondNearest) {
      secondNearest = distanceValue;
    }
  }
  float currentScale = transitionPrismScale();
  return 1.0 - smoothstep(
    max(uBevel * currentScale * 0.55, 0.01 * currentScale),
    max(uBevel * currentScale * 3.8, 0.06 * currentScale),
    secondNearest
  );
}

int nearestPrismPlaneIndex(vec3 point) {
  int nearestIndex = 0;
  float nearestDistance = 1e6;
  for (int index = 0; index < MAX_PRISM_PLANES; index += 1) {
    if (index >= uPrismPlaneCount) break;
    vec3 planeNormal;
    float planeConstant;
    prismPlane(index, planeNormal, planeConstant);
    float distanceValue = abs(planeConstant - dot(planeNormal, point));
    if (distanceValue < nearestDistance) {
      nearestDistance = distanceValue;
      nearestIndex = index;
    }
  }
  return nearestIndex;
}

float morphSilhouette(vec3 point) {
  if (uPrismPlaneCount != 5) return edgeFactor(point);
  float nearestSide = 1e6;
  for (int index = 0; index < MAX_PRISM_PLANES; index += 1) {
    if (index >= uPrismPlaneCount) break;
    vec3 planeNormal;
    float planeConstant;
    prismPlane(index, planeNormal, planeConstant);
    if (abs(planeNormal.z) > 0.999) continue;
    nearestSide = min(nearestSide, abs(planeConstant - dot(planeNormal, point)));
  }
  float currentScale = transitionPrismScale();
  return 1.0 - smoothstep(0.030 * currentScale, 0.14 * currentScale, nearestSide);
}

float fresnelSchlick(float cosine, float ior) {
  float f0 = (ior - 1.0) / (ior + 1.0);
  f0 *= f0;
  return f0 + (1.0 - f0) * pow(1.0 - clamp(cosine, 0.0, 1.0), 5.0);
}

float rectangleMask(vec2 point, vec2 center, vec2 halfSize, float softness) {
  vec2 q = abs(point - center) - halfSize;
  float distanceValue = max(q.x, q.y);
  return 1.0 - smoothstep(0.0, softness, distanceValue);
}

vec3 studioEnvironment(vec3 direction) {
  direction = normalize(direction);
  float longitude = atan(direction.x, direction.z);
  float latitude = asin(clamp(direction.y, -1.0, 1.0));
  vec2 angular = vec2(longitude, latitude);

  vec3 lightColor = mix(
    vec3(0.31),
    vec3(0.77),
    smoothstep(-0.72, 0.75, direction.y)
  );
  vec3 darkColor = mix(
    vec3(0.022),
    vec3(0.11),
    smoothstep(-0.72, 0.75, direction.y)
  );

  float upperSoftbox = rectangleMask(angular, vec2(-0.78, 0.64), vec2(0.54, 0.19), 0.085);
  float sideSoftbox = rectangleMask(angular, vec2(1.15, 0.14), vec2(0.20, 0.60), 0.075);
  float frontSoftbox = rectangleMask(angular, vec2(0.02, -0.08), vec2(0.31, 0.48), 0.055);
  float darkFlag = rectangleMask(angular, vec2(-1.52, 0.02), vec2(0.14, 0.74), 0.045);
  float darkFloor = smoothstep(-0.36, -0.68, direction.y);

  float brightStrip = rectangleMask(angular, vec2(0.58, 0.02), vec2(0.052, 0.72), 0.022);
  float darkStrip = rectangleMask(angular, vec2(0.77, 0.00), vec2(0.045, 0.70), 0.020);
  lightColor = mix(lightColor, vec3(1.0), upperSoftbox * 0.94);
  lightColor = mix(lightColor, vec3(0.97), sideSoftbox * 0.80);
  lightColor = mix(lightColor, vec3(0.88), frontSoftbox * 0.56);
  lightColor = mix(lightColor, vec3(1.0), brightStrip * 0.98);
  lightColor = mix(lightColor, vec3(0.055), darkStrip * 0.96);
  lightColor = mix(lightColor, vec3(0.078), darkFlag * 0.92);
  lightColor = mix(lightColor, vec3(0.14), darkFloor * 0.52);

  // Dark UI has no large white studio surrounding the prism. A dim room and
  // one narrow neutral strip preserve the silhouette without turning the
  // whole dielectric into a silver light source; spectral paths and thin-film
  // sheen are added independently after the environment lookup.
  darkColor = mix(darkColor, vec3(0.20), upperSoftbox * 0.50);
  darkColor = mix(darkColor, vec3(0.15), sideSoftbox * 0.42);
  darkColor = mix(darkColor, vec3(0.11), frontSoftbox * 0.28);
  darkColor = mix(darkColor, vec3(0.42), brightStrip * 0.88);
  darkColor = mix(darkColor, vec3(0.008), darkStrip * 0.96);
  darkColor = mix(darkColor, vec3(0.012), darkFlag * 0.92);
  darkColor = mix(darkColor, vec3(0.025), darkFloor * 0.60);
  return mix(darkColor, lightColor, environmentLightMix());
}

vec3 roughEnvironment(vec3 direction) {
  vec3 sharp = studioEnvironment(direction);
  vec3 average = vec3(mix(0.075, 0.61, environmentLightMix()));
  return mix(sharp, average, clamp(uRoughness, 0.0, 1.0) * 0.64);
}

float channelValue(vec3 color, int channelIndex) {
  if (channelIndex == 0) return color.r;
  if (channelIndex == 1) return color.g;
  return color.b;
}

float traceChannelFromEntry(
  vec3 entryPoint,
  vec3 directionLocal,
  mat3 objectToWorld,
  vec3 entryNormal,
  vec3 reflectedEnvironment,
  float ior,
  int channelIndex,
  out float entryFresnel,
  out float exitFresnel
) {
  float entryCosine = max(dot(-directionLocal, entryNormal), 0.0);
  entryFresnel = fresnelSchlick(entryCosine, ior);

  float reflectedValue = channelValue(reflectedEnvironment, channelIndex);

  vec3 insideDirection = refract(directionLocal, entryNormal, 1.0 / ior);
  if (dot(insideDirection, insideDirection) < 1e-6) {
    exitFresnel = 1.0;
    return reflectedValue;
  }

  vec3 insideOrigin = entryPoint + insideDirection * (0.0015 * renderPrismScale());
  vec3 outgoingDirection = vec3(0.0);
  exitFresnel = 0.0;
  bool escaped = false;

  for (int bounce = 0; bounce < 3; bounce += 1) {
    float innerNear;
    float innerFar;
    vec3 innerNearNormal;
    vec3 innerFarNormal;
    if (!intersectPrism(insideOrigin, insideDirection, innerNear, innerFar, innerNearNormal, innerFarNormal)) break;
    vec3 exitPoint = insideOrigin + insideDirection * innerFar;
    vec3 exitNormal = polishedNormal(exitPoint, innerFarNormal);
    float exitCosine = abs(dot(insideDirection, exitNormal));
    exitFresnel = fresnelSchlick(exitCosine, ior);
    vec3 candidate = refract(insideDirection, -exitNormal, ior);
    if (dot(candidate, candidate) > 1e-6) {
      outgoingDirection = normalize(candidate);
      escaped = true;
      break;
    }
    insideDirection = normalize(reflect(insideDirection, -exitNormal));
    insideOrigin = exitPoint + insideDirection * (0.0015 * renderPrismScale());
  }

  if (!escaped) return reflectedValue;

  vec3 transmittedDirection = objectToWorld * outgoingDirection;
  float transmittedValue = channelValue(roughEnvironment(transmittedDirection), channelIndex);
  float transmittedWeight = (1.0 - entryFresnel) * (1.0 - exitFresnel) * uTransmission;
  float reflectedWeight = clamp(entryFresnel * uFresnelStrength, 0.0, 0.92);
  float normalization = max(transmittedWeight + reflectedWeight, 0.001);
  return (transmittedValue * transmittedWeight + reflectedValue * reflectedWeight) / normalization;
}

float wavelengthForSample(int index) {
  return 420.0 + float(index) * 40.0;
}

int materialAnchorIndex(int sampleIndex, int sampleCount) {
  if (sampleCount <= 3) return sampleIndex * 3;
  if (sampleCount <= 5) {
    if (sampleIndex == 0) return 0;
    if (sampleIndex == 1) return 2;
    if (sampleIndex == 2) return 3;
    if (sampleIndex == 3) return 4;
    return 6;
  }
  return sampleIndex;
}

float materialAnchorWeight(int sampleIndex, int sampleCount) {
  if (sampleCount <= 3) return sampleIndex == 1 ? 3.5 : 1.75;
  if (sampleCount <= 5) return sampleIndex == 2 ? 1.0 : 1.5;
  return 1.0;
}

vec3 spectrumColorForWavelength(float wavelength) {
  // Continuous visible-spectrum approximation used by the reconstructed fan.
  vec3 color = vec3(0.0);
  if (wavelength < 440.0) {
    color = vec3((440.0 - wavelength) / 20.0, 0.0, 1.0);
  } else if (wavelength < 490.0) {
    color = vec3(0.0, (wavelength - 440.0) / 50.0, 1.0);
  } else if (wavelength < 510.0) {
    color = vec3(0.0, 1.0, (510.0 - wavelength) / 20.0);
  } else if (wavelength < 580.0) {
    color = vec3((wavelength - 510.0) / 70.0, 1.0, 0.0);
  } else if (wavelength < 645.0) {
    color = vec3(1.0, (645.0 - wavelength) / 65.0, 0.0);
  } else {
    color = vec3(1.0, 0.0, 0.0);
  }

  float edgeEnergy = wavelength < 440.0
    ? 0.45 + 0.55 * (wavelength - 420.0) / 20.0
    : wavelength > 645.0
      ? 0.75 + 0.25 * (660.0 - wavelength) / 15.0
      : 1.0;
  return color * edgeEnergy;
}

vec3 colorForWavelength(float wavelength) {
  // Anchor samples are normalized so an equal-energy sum reconstructs white.
  return spectrumColorForWavelength(wavelength) / uMaterialSpectralNormalization;
}

float iorForWavelength(float wavelengthNm) {
  // One-term Cauchy dispersion centered on green. The control is a multiplier,
  // while wavelength ordering and curvature remain those of dispersive glass.
  float wavelength = wavelengthNm * 0.001;
  float greenWavelength = 0.540;
  float cauchyB = 0.016;
  float cauchyOffset = cauchyB
    * (1.0 / (wavelength * wavelength) - 1.0 / (greenWavelength * greenWavelength));
  return clamp(uIor + cauchyOffset * max(uDispersion, 0.0), 1.0, 2.8);
}

int opticalLightPathColumn(int lightIndex, int sampleIndex, int anchorIndex) {
  return (lightIndex * MAX_SOURCE_SAMPLES + sampleIndex) * SPECTRAL_ANCHOR_COUNT + anchorIndex;
}

vec4 opticalLightPathTexel(int lightIndex, int sampleIndex, int anchorIndex, int row) {
  return uOpticalPathData[row * OPTICAL_LIGHT_PATH_COLUMNS
    + opticalLightPathColumn(lightIndex, sampleIndex, anchorIndex)];
}

vec4 opticalMaterialPathTexel(int lightIndex, int anchorIndex, int row) {
  return uOpticalPathData[OPTICAL_MATERIAL_PATH_OFFSET
    + row * OPTICAL_MATERIAL_PATH_COLUMNS
    + lightIndex * SPECTRAL_ANCHOR_COUNT
    + anchorIndex];
}

bool loadOpticalLightPath(
  int lightIndex,
  int sampleIndex,
  int anchorIndex,
  out vec3 entryWorld,
  out vec3 exitWorld,
  out vec3 outgoingWorld
) {
  vec4 entry = opticalLightPathTexel(lightIndex, sampleIndex, anchorIndex, 0);
  vec4 exitAndLengthA = opticalLightPathTexel(lightIndex, sampleIndex, anchorIndex, 1);
  vec4 outgoingAndLengthB = opticalLightPathTexel(lightIndex, sampleIndex, anchorIndex, 2);
  entryWorld = entry.xyz;
  exitWorld = exitAndLengthA.xyz;
  outgoingWorld = outgoingAndLengthB.xyz;
  return entry.w > 0.5;
}

bool loadOpticalMaterialPath(
  int lightIndex,
  int anchorIndex,
  out vec3 exitWorld,
  out vec3 outgoingWorld,
  out vec3 internalOriginA,
  out vec3 internalDirectionA,
  out float internalLengthA,
  out vec3 internalOriginB,
  out vec3 internalDirectionB,
  out float internalLengthB
) {
  vec4 exitAndLengthA = opticalMaterialPathTexel(lightIndex, anchorIndex, 0);
  vec4 outgoingAndLengthB = opticalMaterialPathTexel(lightIndex, anchorIndex, 1);
  vec4 originAAndValid = opticalMaterialPathTexel(lightIndex, anchorIndex, 2);
  exitWorld = exitAndLengthA.xyz;
  outgoingWorld = outgoingAndLengthB.xyz;
  internalLengthA = exitAndLengthA.w;
  internalLengthB = outgoingAndLengthB.w;
  internalOriginA = originAAndValid.xyz;
  internalDirectionA = opticalMaterialPathTexel(lightIndex, anchorIndex, 3).xyz;
  internalOriginB = opticalMaterialPathTexel(lightIndex, anchorIndex, 4).xyz;
  internalDirectionB = opticalMaterialPathTexel(lightIndex, anchorIndex, 5).xyz;
  return originAAndValid.w > 0.5;
}

bool traceTransmittedViewRayFromInside(
  vec3 insideOrigin,
  vec3 insideDirection,
  float firstExitDistance,
  vec3 firstExitNormal,
  mat3 objectToWorld,
  float ior,
  out vec3 exitWorld,
  out vec3 outgoingWorld
) {
  exitWorld = vec3(0.0);
  outgoingWorld = vec3(0.0);
  float innerFar = firstExitDistance;
  vec3 innerFarNormal = firstExitNormal;

  for (int bounce = 0; bounce < 3; bounce += 1) {
    vec3 exitPoint = insideOrigin + insideDirection * innerFar;
    vec3 exitNormal = polishedNormal(exitPoint, innerFarNormal);
    vec3 candidate = refract(insideDirection, -exitNormal, ior);
    if (dot(candidate, candidate) > 1e-6) {
      exitWorld = objectToWorld * exitPoint;
      outgoingWorld = normalize(objectToWorld * candidate);
      return true;
    }
    insideDirection = normalize(reflect(insideDirection, -exitNormal));
    insideOrigin = exitPoint + insideDirection * (0.0015 * renderPrismScale());
    if (bounce >= 2) break;
    float innerNear;
    vec3 innerNearNormal;
    if (!intersectPrism(
      insideOrigin,
      insideDirection,
      innerNear,
      innerFar,
      innerNearNormal,
      innerFarNormal
    )) break;
  }
  return false;
}

float henyeyGreenstein(float cosine, float anisotropy) {
  float g2 = anisotropy * anisotropy;
  float denominator = max(1.0 + g2 - 2.0 * anisotropy * cosine, 0.001);
  return (1.0 - g2) / pow(denominator, 1.5);
}

vec2 beamSingleScattering(
  vec3 cameraOrigin,
  vec3 cameraDirection,
  float cameraLength,
  vec3 beamOrigin,
  vec3 beamDirection,
  float beamLength,
  float distanceAtOrigin,
  float distanceDirection,
  float scatteringStrength,
  float radiusAtOrigin,
  float radiusAtEnd
) {
  // Analytic line integral through a finite Gaussian beam volume. The closest
  // point locates the local cross-section; the view-angle term integrates the
  // optical path through that cross-section instead of drawing a 2D line.
  vec3 separation = cameraOrigin - beamOrigin;
  float directionDot = dot(cameraDirection, beamDirection);
  float cameraDot = dot(cameraDirection, separation);
  float beamDot = dot(beamDirection, separation);
  float denominator = max(1.0 - directionDot * directionDot, 0.0001);
  float beamDistance = clamp((beamDot - directionDot * cameraDot) / denominator, 0.0, beamLength);
  vec3 beamPoint = beamOrigin + beamDirection * beamDistance;
  float cameraDistance = clamp(dot(beamPoint - cameraOrigin, cameraDirection), 0.0, cameraLength);
  vec3 cameraPoint = cameraOrigin + cameraDirection * cameraDistance;
  float radialDistance = length(cameraPoint - beamPoint);

  float distanceFromPrism = max(distanceAtOrigin + beamDistance * distanceDirection, 0.0);
  float atmosphericClarity = exp(-distanceFromPrism / max(uScaledScatteringFalloff, 0.02));
  float segmentPosition = beamDistance / max(beamLength, 0.001);
  float physicalRadius = mix(radiusAtOrigin, radiusAtEnd, segmentPosition) * uBeamWidth * renderPrismScale();
  float scatteringRadius = max(physicalRadius * (0.72 + atmosphericClarity * 1.18), 0.005);
  float normalizedCore = radialDistance / scatteringRadius;
  float normalizedHalo = radialDistance / (scatteringRadius * 2.7);
  float coreDensity = exp(-0.5 * normalizedCore * normalizedCore);
  float haloDensity = exp(-0.5 * normalizedHalo * normalizedHalo) * 0.13;

  float localDensity = scatteringStrength * atmosphericClarity;
  float scatteringAngle = dot(beamDirection, -cameraDirection);
  float phase = henyeyGreenstein(scatteringAngle, 0.32) * 0.34;
  float gaussianPathLength = sqrt(6.2831853) * scatteringRadius / sqrt(denominator);
  // The analytic Gaussian integral assumes an infinite cylinder and therefore
  // diverges as the camera ray becomes parallel to the beam. The optical path
  // is finite: limit it by both the illuminated segment and the distance over
  // which this atmosphere remains visible.
  float finiteSegmentLength = min(cameraLength, beamLength) + scatteringRadius * 2.0;
  float attenuationLimitedLength = max(
    uScaledScatteringFalloff * 2.0,
    scatteringRadius * 4.0
  );
  float pathLength = min(gaussianPathLength, min(
    finiteSegmentLength,
    attenuationLimitedLength
  ));
  float opticalScale = localDensity * phase * pathLength * 9.0;
  return vec2(coreDensity, haloDensity) * opticalScale;
}

float cross2D(vec2 first, vec2 second) {
  return first.x * second.y - first.y * second.x;
}

vec3 spinnerVertex(int index) {
  if (index == 0) return vec3(1.0, 0.0, 0.0);
  if (index == 1) return vec3(-1.0, 0.0, 0.0);
  if (index == 2) return vec3(0.0, 1.0, 0.0);
  if (index == 3) return vec3(0.0, -1.0, 0.0);
  if (index == 4) return vec3(0.0, 0.0, 1.0);
  return vec3(0.0, 0.0, -1.0);
}

ivec3 spinnerFace(int index) {
  if (index == 0) return ivec3(4, 0, 2);
  if (index == 1) return ivec3(2, 0, 5);
  if (index == 2) return ivec3(3, 0, 4);
  if (index == 3) return ivec3(5, 0, 3);
  if (index == 4) return ivec3(2, 1, 4);
  if (index == 5) return ivec3(5, 1, 2);
  if (index == 6) return ivec3(4, 1, 3);
  return ivec3(3, 1, 5);
}

ivec4 spinnerEdge(int index) {
  if (index == 0) return ivec4(0, 4, 0, 2);
  if (index == 1) return ivec4(0, 2, 0, 1);
  if (index == 2) return ivec4(2, 4, 0, 4);
  if (index == 3) return ivec4(0, 5, 1, 3);
  if (index == 4) return ivec4(2, 5, 1, 5);
  if (index == 5) return ivec4(0, 3, 2, 3);
  if (index == 6) return ivec4(3, 4, 2, 6);
  if (index == 7) return ivec4(3, 5, 3, 7);
  if (index == 8) return ivec4(1, 2, 4, 5);
  if (index == 9) return ivec4(1, 4, 4, 6);
  if (index == 10) return ivec4(1, 5, 5, 7);
  return ivec4(1, 3, 6, 7);
}

float spinnerSegmentDistance(vec2 point, vec2 first, vec2 second) {
  vec2 segment = second - first;
  float amount = clamp(dot(point - first, segment) / max(dot(segment, segment), 0.000001), 0.0, 1.0);
  return length(point - mix(first, second, amount));
}

vec3 spinnerSpectrumStops(float phase, bool sheen) {
  float value = fract(phase);
  vec3 coral = vec3(1.0, 0.6745, 0.7294);
  vec3 amber = vec3(1.0, 0.8275, 0.5529);
  vec3 yellow = vec3(1.0, 0.9608, 0.6157);
  vec3 lime = vec3(0.7373, 0.9569, 0.6706);
  vec3 cyan = vec3(0.2863, 0.9059, 0.9451);
  vec3 blue = vec3(0.4, 0.6824, 1.0);
  vec3 indigo = vec3(0.4431, 0.4941, 0.9569);
  vec3 violet = vec3(0.7255, 0.5608, 1.0);
  vec3 rose = vec3(0.9686, 0.7098, 0.8902);
  vec3 pearl = vec3(0.8118, 0.9098, 0.9451);
  if (sheen) {
    if (value < 0.14) return mix(coral, amber, value / 0.14);
    if (value < 0.28) return mix(amber, lime, (value - 0.14) / 0.14);
    if (value < 0.43) return mix(lime, cyan, (value - 0.28) / 0.15);
    if (value < 0.58) return mix(cyan, blue, (value - 0.43) / 0.15);
    if (value < 0.69) return mix(blue, pearl, (value - 0.58) / 0.11);
    if (value < 0.80) return mix(pearl, violet, (value - 0.69) / 0.11);
    if (value < 0.90) return mix(violet, rose, (value - 0.80) / 0.10);
    return mix(rose, coral, (value - 0.90) / 0.10);
  }
  if (value < 0.10) return mix(coral, amber, value / 0.10);
  if (value < 0.18) return mix(amber, yellow, (value - 0.10) / 0.08);
  if (value < 0.30) return mix(yellow, lime, (value - 0.18) / 0.12);
  if (value < 0.44) return mix(lime, cyan, (value - 0.30) / 0.14);
  if (value < 0.57) return mix(cyan, blue, (value - 0.44) / 0.13);
  if (value < 0.70) return mix(blue, indigo, (value - 0.57) / 0.13);
  if (value < 0.82) return mix(indigo, violet, (value - 0.70) / 0.12);
  if (value < 0.92) return mix(violet, rose, (value - 0.82) / 0.10);
  return mix(rose, coral, (value - 0.92) / 0.08);
}

float spinnerSourcePhase() {
  return uPhase - 0.62 / (2.0 * PI);
}

float spinnerSourceWorldRotationPhase(float worldPhase) {
  return worldPhase;
}

vec3 spinnerSpectralSample(float phase, float richness, float warmth, bool edge, bool sheen) {
  vec3 pearl = vec3(0.8118, 0.9098, 0.9451);
  vec3 color = spinnerSpectrumStops(phase, sheen);
  float value = fract(phase);
  float warmDistance = min(value, 1.0 - value);
  float warmBand = 1.0 - smoothstep(0.04, 0.24, warmDistance);
  color = mix(color, vec3(0.9882, 1.0, 1.0), warmBand * (1.0 - warmth) * 0.2);
  float chroma = clamp(
    (edge ? 0.5 : 0.28) + richness * (edge ? 0.34 : 0.38),
    0.0,
    edge ? 0.94 : 0.78
  );
  return mix(pearl, color, chroma);
}

vec3 spinnerNormalizeLuma(vec3 color, float target) {
  float luminance = dot(color, vec3(0.2126, 0.7152, 0.0722));
  return clamp(color * target / max(luminance, 0.00001), 0.0, 1.0);
}

vec3 spinnerSpectralEnvironment(
  vec3 direction,
  float richness,
  float warmth,
  float bias,
  bool edge,
  out float energy,
  out float hue
) {
  float azimuth = atan(direction.z, direction.x) / (2.0 * PI);
  float elevation = asin(clamp(direction.y, -1.0, 1.0)) / PI;
  hue = fract(azimuth * (edge ? 2.0 : 1.0) + 0.5 + elevation * (edge ? 0.34 : 0.26) + bias);
  energy = clamp(
    (edge ? 0.46 : 0.28) + richness * (edge ? 0.34 : 0.30) + abs(direction.y) * 0.12,
    0.0,
    1.0
  );
  return spinnerSpectralSample(hue, richness, warmth, edge, false);
}

float spinnerEdgeStyleDevelopment(float edgeDevelopment) {
  return smoothstep(0.72, 1.0, clamp(edgeDevelopment, 0.0, 1.0));
}

float prismEdgeActivation(int edgeIndex, int bestEdgeIndex, float edgeDevelopment) {
  float progress = clamp(edgeDevelopment / 0.72, 0.0, 1.0);
  float order = fract(float(edgeIndex) * 0.61803398875 + 0.17);
  float threshold = edgeIndex == bestEdgeIndex ? 0.0 : 0.12 + order * 0.46;
  float activation = smoothstep(threshold, min(threshold + 0.24, 0.90), progress);
  return edgeDevelopment * activation;
}

float spinnerCreaseActivation(int edgeIndex, int bestCreaseIndex, float creaseDevelopment) {
  float order = fract(float(edgeIndex) * 0.61803398875 + 0.31);
  float threshold = edgeIndex == bestCreaseIndex ? 0.0 : 0.10 + order * 0.50;
  return smoothstep(threshold, min(threshold + 0.28, 0.94), clamp(creaseDevelopment, 0.0, 1.0));
}

vec3 bridgeSpectralEdgeColor(vec2 start, vec2 end) {
  vec2 tangent = normalize(end - start);
  if (tangent.x < 0.0 || (abs(tangent.x) < 0.00001 && tangent.y < 0.0)) tangent *= -1.0;
  float tangentPhase = atan(tangent.y, tangent.x) / PI;
  vec2 midpoint = (start + end) * 0.5;
  float positionPhase = atan(midpoint.y, midpoint.x) / (2.0 * PI);
  float phase = fract(0.5 + tangentPhase + positionPhase * 0.12 + spinnerSourcePhase() * 0.18);
  return spinnerSpectralSample(phase, 0.94, 0.18, true, false);
}

vec3 bridgePrismVertex(int index) {
  float prismX = 0.735 / (sqrt(3.0) * 0.5);
  if (index == 0) return vec3(prismX, -0.49, -0.58);
  if (index == 1) return vec3(-prismX, -0.49, 0.58);
  if (index == 2) return vec3(0.0, 0.98, -0.58);
  if (index == 3) return vec3(prismX, -0.49, 0.58);
  if (index == 4) return vec3(0.0, 0.98, 0.58);
  return vec3(-prismX, -0.49, -0.58);
}

vec3 bridgeVertex(int index) {
  return mix(bridgePrismVertex(index), spinnerVertex(index) * 0.98, uSpinnerMorph.z);
}

ivec3 bridgePrismFace(int index) {
  if (index == 0) return ivec3(0, 2, 5);
  if (index == 1) return ivec3(1, 4, 3);
  if (index == 2) return ivec3(0, 5, 1);
  if (index == 3) return ivec3(0, 3, 4);
  return ivec3(2, 4, 1);
}

ivec4 bridgePrismEdge(int index) {
  if (index == 0) return ivec4(0, 2, 0, 3);
  if (index == 1) return ivec4(2, 5, 0, 4);
  if (index == 2) return ivec4(5, 0, 0, 2);
  if (index == 3) return ivec4(3, 4, 1, 3);
  if (index == 4) return ivec4(4, 1, 1, 4);
  if (index == 5) return ivec4(1, 3, 1, 2);
  if (index == 6) return ivec4(0, 3, 2, 3);
  if (index == 7) return ivec4(2, 4, 3, 4);
  return ivec4(5, 1, 2, 4);
}

vec2 prismIconProjectVertex(
  vec3 vertex,
  mat3 objectToWorld,
  float cameraDistance,
  float projectionDistance
) {
  vec3 transformed = objectToWorld * (vertex * transitionPrismScale());
  float depth = max(cameraDistance - transformed.z, 0.000001);
  return transformed.xy * projectionDistance / depth;
}

float prismIconCoverage(float distanceValue, float halfWidth, float antialiasWidth) {
  return 1.0 - smoothstep(
    max(halfWidth - antialiasWidth * 0.5, 0.0),
    halfWidth + antialiasWidth * 0.5,
    distanceValue
  );
}

float prismIconEdgeHalfWidth(float cssPixelWidth, float edgeWidthCssPixels) {
  return cssPixelWidth * max(edgeWidthCssPixels, 0.0) * 0.5;
}

ivec3 prismIconFace(int index) {
  if (index == 0) return ivec3(0, 5, 2);
  if (index == 1) return ivec3(1, 3, 4);
  if (index == 2) return ivec3(0, 2, 4);
  if (index == 3) return ivec3(5, 1, 4);
  return ivec3(0, 3, 1);
}

ivec4 prismIconEdge(int index) {
  if (index == 0) return ivec4(0, 2, 0, 2);
  if (index == 1) return ivec4(2, 5, 0, 3);
  if (index == 2) return ivec4(5, 0, 0, 4);
  if (index == 3) return ivec4(1, 3, 1, 4);
  if (index == 4) return ivec4(3, 4, 1, 2);
  if (index == 5) return ivec4(4, 1, 1, 3);
  if (index == 6) return ivec4(0, 3, 2, 4);
  if (index == 7) return ivec4(2, 4, 2, 3);
  return ivec4(5, 1, 3, 4);
}

float prismIconEdgeHighlight(
  vec2 point,
  vec2 start,
  vec2 end,
  int edgeIndex,
  float progress
) {
  vec2 edgeVector = end - start;
  float amount = clamp(dot(point - start, edgeVector) / max(dot(edgeVector, edgeVector), 0.000001), 0.0, 1.0);
  float objectEdgeCoordinate = (float(edgeIndex) + amount) / 9.0;
  float center = mix(-0.08, 1.08, clamp(progress, 0.0, 1.0));
  float band = 1.0 - smoothstep(0.035, 0.12, abs(objectEdgeCoordinate - center));
  return band * sin(PI * clamp(progress, 0.0, 1.0));
}

void prismActivationPass(
  vec3 objectPoint,
  float progress,
  out float whiteBand,
  out float exitSpectrum
) {
  float amount = clamp(progress, 0.0, 1.0);
  whiteBand = 0.0;
  exitSpectrum = 0.0;
  if (amount <= 0.0001 || amount >= 0.9999) return;
  float coordinate = dot(
    objectPoint / max(transitionPrismScale(), 0.000001),
    normalize(vec3(0.72, 0.24, 0.65))
  ) * 0.28 + 0.5;
  float center = mix(-0.10, 1.10, amount);
  float band = 1.0 - smoothstep(0.05, 0.18, abs(coordinate - center));
  float pulseEnvelope = sin(PI * amount);
  float exitGate = smoothstep(0.82, 0.92, center);
  whiteBand = band * pulseEnvelope;
  exitSpectrum = whiteBand * exitGate;
}

vec4 prismIconEdgeSample(
  vec2 point,
  mat3 objectToWorld,
  float cameraDistance,
  float projectionDistance,
  float edgeWidthCssPixels,
  float edgeAlpha,
  float edgeHighlightProgress,
  out vec3 hdrRadiance
) {
  vec3 transformed[6];
  vec2 projected[6];
  for (int index = 0; index < 6; index += 1) {
    vec3 vertex = bridgePrismVertex(index);
    transformed[index] = objectToWorld * (vertex * transitionPrismScale());
    projected[index] = prismIconProjectVertex(vertex, objectToWorld, cameraDistance, projectionDistance);
  }
  float faceFront[5];
  for (int faceIndex = 0; faceIndex < 5; faceIndex += 1) {
    ivec3 face = prismIconFace(faceIndex);
    vec3 normal = normalize(cross(
      transformed[face.y] - transformed[face.x],
      transformed[face.z] - transformed[face.x]
    ));
    vec3 faceCenter = (transformed[face.x] + transformed[face.y] + transformed[face.z]) / 3.0;
    faceFront[faceIndex] = dot(normal, vec3(0.0, 0.0, cameraDistance) - faceCenter) > 0.0 ? 1.0 : 0.0;
  }
  float cssPixelWidth = max(max(fwidth(point.x), fwidth(point.y)) * uCssPixelRatio, 0.00001);
  float alpha = 0.0;
  float highlight = 0.0;
  for (int edgeIndex = 0; edgeIndex < 9; edgeIndex += 1) {
    ivec4 edge = prismIconEdge(edgeIndex);
    bool silhouette = faceFront[edge.z] != faceFront[edge.w];
    bool frontCrease = faceFront[edge.z] > 0.5 && faceFront[edge.w] > 0.5;
    if (!silhouette && !frontCrease) continue;
    float coverage = prismIconCoverage(
      spinnerSegmentDistance(point, projected[edge.x], projected[edge.y]),
      prismIconEdgeHalfWidth(cssPixelWidth, edgeWidthCssPixels),
      cssPixelWidth
    );
    alpha = max(alpha, coverage);
    highlight = max(highlight, coverage * prismIconEdgeHighlight(
      point,
      projected[edge.x],
      projected[edge.y],
      (edgeIndex + 3) % 9,
      edgeHighlightProgress
    ));
  }
  alpha *= uIconEdgeColor.a * clamp(edgeAlpha, 0.0, 1.0);
  float highlightAlpha = highlight * uIconEdgeColor.a * mix(0.56, 0.30, uDarkMode);
  float outputAlpha = max(alpha, highlightAlpha);
  vec3 highlightColor = vec3(1.0);
  float highlightMix = highlight * mix(0.84, 0.34, uDarkMode);
  vec3 edgeColor = mix(uIconEdgeColor.rgb, highlightColor, highlightMix);
  hdrRadiance = vec3(1.0) * highlight * uIconEdgeColor.a * 0.80;
  return vec4(edgeColor * outputAlpha, outputAlpha);
}

float spinnerFacetPhase(int faceIndex) {
  if (faceIndex == 0) return 0.92;
  if (faceIndex == 1) return 0.57;
  if (faceIndex == 2) return 0.10;
  if (faceIndex == 3) return 0.70;
  if (faceIndex == 4) return 0.0;
  if (faceIndex == 5) return 0.44;
  if (faceIndex == 6) return 0.82;
  return 0.30;
}

void spinnerSurfaceMaterial(
  vec3 outwardNormal,
  int faceIndex,
  float surfaceDevelopment,
  out vec3 faceColor,
  out float faceAlpha,
  out vec3 faceHdrRadiance
) {
  vec3 view = vec3(0.0, 0.0, 1.0);
  vec3 incident = vec3(0.0, 0.0, -1.0);
  float viewDot = dot(outwardNormal, view);
  bool topologyFront = viewDot >= 0.0;
  vec3 viewNormal = topologyFront ? outwardNormal : -outwardNormal;
  float richness = 1.35;
  float warmth = 0.18;
  float transmission = 0.30;
  float restOpacity = 0.80;
  float shineStrength = 1.45;
  float interior = 0.85;
  float diffuse = max(dot(viewNormal, normalize(vec3(-0.58, 0.82, 1.1))), 0.0);
  float fillDiffuse = max(dot(viewNormal, normalize(vec3(0.74, 0.12, 0.92))), 0.0);
  float broad = clamp((diffuse * 0.64 + fillDiffuse * 0.30) * 1.28, 0.0, 1.0);
  float fresnel = 0.04 + 0.96 * pow(1.0 - clamp(abs(viewDot), 0.0, 1.0), 5.0);
  float spinnerPhase = spinnerSourcePhase();
  float sheenAngle = spinnerPhase * 2.0 * PI * -3.0 + 2.0 * PI * 0.62;
  vec3 sheenHalf = normalize(normalize(vec3(cos(sheenAngle) * 1.35, 0.18 + sin(sheenAngle) * 1.8, 1.0)) + view);
  float opticalSheen = pow(max(dot(viewNormal, sheenHalf), 0.0), 4.5);
  float flash = clamp(opticalSheen * (0.82 + fresnel * 0.12), 0.0, 1.0);
  vec3 reflected = normalize(reflect(incident, viewNormal));
  vec3 darkDirection = normalize(vec3(0.96, -0.12, 0.24));
  float dark = pow(max(dot(reflected, darkDirection), 0.0), 3.8);
  float spectralEnergy;
  float spectralHue;
  vec3 spectralColor = spinnerSpectralEnvironment(
    reflected,
    richness,
    warmth,
    float(faceIndex) * 0.013,
    false,
    spectralEnergy,
    spectralHue
  );
  vec3 sheenColor = spinnerNormalizeLuma(
    spinnerSpectralSample(
      fract(spinnerPhase * 3.0 + spinnerFacetPhase(faceIndex)),
      richness,
      warmth,
      false,
      true
    ),
    0.72
  );
  float sheenPresence = smoothstep(0.08, 0.34, opticalSheen);
  float spectralStrength = clamp(
    0.08 + opticalSheen * (
      (0.18 + richness * 0.30) * (0.58 + spectralEnergy * 0.42)
        + fresnel * 0.08
    ),
    0.0,
    0.76
  );
  vec3 developedColor = mix(
    vec3(0.7216, 0.8353, 0.8902),
    mix(spectralColor, sheenColor, sheenPresence),
    spectralStrength
  );
  developedColor = mix(
    developedColor,
    vec3(0.9882, 1.0, 1.0),
    clamp((flash * 0.10 + opticalSheen * 0.02) * shineStrength, 0.0, 1.0)
  );
  float darkenAmount = clamp(
    (1.0 - broad) * (topologyFront ? 0.055 : 0.13) + dark * 0.055,
    0.0,
    1.0
  );
  developedColor = mix(
    developedColor,
    vec3(0.1725, 0.2157, 0.3843),
    darkenAmount * (1.0 - flash * 0.72)
  );
  developedColor = mix(
    vec3(0.8118, 0.9098, 0.9451),
    developedColor,
    topologyFront ? 0.76 + opticalSheen * 0.16 : 0.64
  );
  faceColor = developedColor;
  float restingAlpha = topologyFront
    ? 0.116 + broad * 0.055 + spectralEnergy * 0.10 + dark * 0.09 - transmission * 0.18
    : 0.023 + transmission * 0.22 + interior * 0.06 + spectralEnergy * 0.045;
  float highlightAlpha = topologyFront
    ? opticalSheen * fresnel * 0.13 + flash * (0.70 * 0.82 + 0.04) + opticalSheen * 0.04
    : opticalSheen * fresnel * 0.045 + flash * 0.055;
  faceAlpha = topologyFront
    ? clamp(restingAlpha * restOpacity + highlightAlpha * shineStrength, 0.018, 0.94)
    : clamp(restingAlpha * restOpacity + highlightAlpha * min(shineStrength, 1.0), 0.012, 0.42);
  // The topology exchange happens under an edge-dominant spinner material.
  // Developing the facets only after that frame hides the discrete solid swap
  // without crossfading the two geometries or briefly restoring milky glass.
  faceAlpha *= mix(0.06, 1.0, smoothstep(0.0, 1.0, surfaceDevelopment));
  float faceHdrEnergy = topologyFront
    ? clamp(flash * (0.72 + shineStrength * 0.28), 0.0, 1.0) * surfaceDevelopment
    : 0.0;
  float faceHdrPresence = smoothstep(0.08, 0.62, faceHdrEnergy);
  vec3 faceHdrColor = mix(
    mix(spectralColor, sheenColor, sheenPresence),
    vec3(0.9882, 1.0, 1.0),
    flash * 0.36
  );
  // The source spinner promotes only moving facet highlights, not the entire
  // translucent face. Keeping the live bridge equally sparse removes the HDR
  // brightness step when ownership moves to the prerendered asset.
  faceHdrRadiance = faceHdrColor * faceHdrPresence * (0.42 + flash * 0.88);
}

vec4 prismBridgeEdgeSample(
  vec2 point,
  mat3 objectToWorld,
  vec3 cameraOrigin,
  float projectionDistance,
  float edgeDevelopment,
  out vec3 hdrRadiance
) {
  vec3 transformed[6];
  vec2 projected[6];
  float currentScale = transitionPrismScale();
  for (int vertexIndex = 0; vertexIndex < 6; vertexIndex += 1) {
    transformed[vertexIndex] = objectToWorld * (bridgePrismVertex(vertexIndex) * currentScale);
    float depth = max(cameraOrigin.z - transformed[vertexIndex].z, 0.02);
    projected[vertexIndex] = (transformed[vertexIndex].xy - cameraOrigin.xy) * projectionDistance / depth;
  }

  float faceFront[5];
  vec3 faceNormal[5];
  vec3 view = vec3(0.0, 0.0, 1.0);
  vec3 incident = vec3(0.0, 0.0, -1.0);
  for (int faceIndex = 0; faceIndex < 5; faceIndex += 1) {
    ivec3 indices = bridgePrismFace(faceIndex);
    vec3 first = transformed[indices.x];
    vec3 second = transformed[indices.y];
    vec3 third = transformed[indices.z];
    vec3 center = (first + second + third) / 3.0;
    vec3 normal = normalize(cross(second - first, third - first));
    if (dot(normal, center) < 0.0) normal *= -1.0;
    faceNormal[faceIndex] = normal;
    faceFront[faceIndex] = dot(normal, view) >= 0.0 ? 1.0 : 0.0;
  }

  int bestEdgeIndex = -1;
  float bestEdgeScore = -1.0;
  int bestCreaseIndex = -1;
  float bestCreaseScore = -1.0;
  for (int edgeIndex = 0; edgeIndex < 9; edgeIndex += 1) {
    ivec4 edge = bridgePrismEdge(edgeIndex);
    bool silhouette = faceFront[edge.z] != faceFront[edge.w];
    bool frontCrease = faceFront[edge.z] > 0.5 && faceFront[edge.w] > 0.5;
    if (!silhouette && !frontCrease) continue;
    vec3 normal = normalize(faceNormal[edge.z] + faceNormal[edge.w]);
    if (dot(normal, view) < 0.0) normal *= -1.0;
    float fresnel = pow(1.0 - clamp(abs(dot(normal, view)), 0.0, 1.0), 2.25);
    vec3 reflected = normalize(reflect(incident, normal));
    float energy;
    float hue;
    spinnerSpectralEnvironment(reflected, 0.94, 0.18, 0.0, true, energy, hue);
    float score = fresnel * 0.54 + energy * 0.36
      + length(projected[edge.y] - projected[edge.x]) * 0.08;
    if (silhouette && score > bestEdgeScore) {
      bestEdgeScore = score;
      bestEdgeIndex = edgeIndex;
    }
    if (frontCrease && score > bestCreaseScore) {
      bestCreaseScore = score;
      bestCreaseIndex = edgeIndex;
    }
  }

  vec4 result = vec4(0.0);
  float cssPixelWidth = max(max(fwidth(point.x), fwidth(point.y)) * uCssPixelRatio, 0.00001);
  for (int edgeIndex = 0; edgeIndex < 9; edgeIndex += 1) {
    ivec4 edge = bridgePrismEdge(edgeIndex);
    bool silhouette = faceFront[edge.z] != faceFront[edge.w];
    bool frontCrease = faceFront[edge.z] > 0.5 && faceFront[edge.w] > 0.5;
    if (!silhouette && !frontCrease) continue;
    vec3 normal = normalize(faceNormal[edge.z] + faceNormal[edge.w]);
    if (dot(normal, view) < 0.0) normal *= -1.0;
    vec3 reflected = normalize(reflect(incident, normal));
    vec2 start = projected[edge.x];
    vec2 end = projected[edge.y];
    vec3 edgeColor = bridgeSpectralEdgeColor(start, end);
    float halfWidth = (silhouette ? 0.74 : 0.50) * cssPixelWidth * 0.5;
    float coverage = 1.0 - smoothstep(
      max(halfWidth - cssPixelWidth * 0.5, 0.0),
      halfWidth + cssPixelWidth * 0.5,
      spinnerSegmentDistance(point, start, end)
    );
    float development = silhouette
      ? prismEdgeActivation(edgeIndex, bestEdgeIndex, edgeDevelopment)
      : spinnerCreaseActivation(edgeIndex, bestCreaseIndex, uSpinnerCreaseMorph);
    float sourceAlpha = coverage * (silhouette ? 0.94 : 0.58) * development;
    result.rgb = edgeColor * sourceAlpha + result.rgb * (1.0 - sourceAlpha);
    result.a = sourceAlpha + result.a * (1.0 - sourceAlpha);
  }
  hdrRadiance = result.rgb * edgeDevelopment * 1.55;
  return result;
}

vec4 bridgeEdgeSample(
  vec2 point,
  mat3 objectToWorld,
  vec3 cameraOrigin,
  float projectionDistance,
  out vec3 hdrRadiance
) {
  float edgeDevelopment = uSpinnerMaterialMorph.x;
  hdrRadiance = vec3(0.0);
  if (edgeDevelopment <= 0.0001) return vec4(0.0);
  if (uSpinnerMorph.z < 0.5) {
    return prismBridgeEdgeSample(
      point,
      objectToWorld,
      cameraOrigin,
      projectionDistance,
      edgeDevelopment,
      hdrRadiance
    );
  }
  vec3 transformed[6];
  vec2 projected[6];
  float currentScale = transitionPrismScale();
  for (int vertexIndex = 0; vertexIndex < 6; vertexIndex += 1) {
    transformed[vertexIndex] = objectToWorld * (bridgeVertex(vertexIndex) * currentScale);
    float depth = max(cameraOrigin.z - transformed[vertexIndex].z, 0.02);
    projected[vertexIndex] = (transformed[vertexIndex].xy - cameraOrigin.xy) * projectionDistance / depth;
  }
  float faceFront[8];
  vec3 faceNormal[8];
  float faceSheen[8];
  vec3 view = vec3(0.0, 0.0, 1.0);
  vec3 incident = vec3(0.0, 0.0, -1.0);
  float spinnerPhase = spinnerSourcePhase();
  float sheenAngle = spinnerPhase * 2.0 * PI * -3.0 + 2.0 * PI * 0.62;
  vec3 sheenHalf = normalize(normalize(vec3(cos(sheenAngle) * 1.35, 0.18 + sin(sheenAngle) * 1.8, 1.0)) + view);
  for (int faceIndex = 0; faceIndex < 8; faceIndex += 1) {
    ivec3 indices = spinnerFace(faceIndex);
    vec3 first = transformed[indices.x];
    vec3 second = transformed[indices.y];
    vec3 third = transformed[indices.z];
    vec3 center = (first + second + third) / 3.0;
    vec3 normal = normalize(cross(second - first, third - first));
    if (dot(normal, center) < 0.0) normal *= -1.0;
    faceNormal[faceIndex] = normal;
    faceFront[faceIndex] = dot(normal, view) >= 0.0 ? 1.0 : 0.0;
    vec3 viewNormal = faceFront[faceIndex] > 0.5 ? normal : -normal;
    faceSheen[faceIndex] = pow(max(dot(viewNormal, sheenHalf), 0.0), 4.5);
  }

  int bestCreaseIndex = -1;
  float bestCreaseScore = -1.0;
  for (int edgeIndex = 0; edgeIndex < 12; edgeIndex += 1) {
    ivec4 edge = spinnerEdge(edgeIndex);
    bool silhouette = faceFront[edge.z] != faceFront[edge.w];
    bool frontCrease = faceFront[edge.z] > 0.5 && faceFront[edge.w] > 0.5;
    if (silhouette || !frontCrease) continue;
    vec3 normal = normalize(faceNormal[edge.z] + faceNormal[edge.w]);
    if (dot(normal, view) < 0.0) normal *= -1.0;
    float fresnel = pow(1.0 - clamp(abs(dot(normal, view)), 0.0, 1.0), 2.25);
    vec3 reflected = normalize(reflect(incident, normal));
    float white = pow(max(dot(reflected, normalize(vec3(-0.18, 0.86, 0.48))), 0.0), 14.0);
    float energy;
    float hue;
    spinnerSpectralEnvironment(reflected, 1.458, 0.18, 0.0, true, energy, hue);
    float score = 0.28 + fresnel * 0.17 + white * 0.22 + energy * 0.22
      + max(faceSheen[edge.z], faceSheen[edge.w]) * 0.36;
    if (score > bestCreaseScore) {
      bestCreaseScore = score;
      bestCreaseIndex = edgeIndex;
    }
  }

  vec4 result = vec4(0.0);
  float creaseDevelopment = uSpinnerCreaseMorph;
  float cssPixelWidth = max(max(fwidth(point.x), fwidth(point.y)) * uCssPixelRatio, 0.00001);
  for (int edgeIndex = 0; edgeIndex < 12; edgeIndex += 1) {
    ivec4 edge = spinnerEdge(edgeIndex);
    bool silhouette = faceFront[edge.z] != faceFront[edge.w];
    bool frontCrease = faceFront[edge.z] > 0.5 && faceFront[edge.w] > 0.5;
    bool crease = frontCrease && bestCreaseScore > 0.29;
    if (!silhouette && !crease) continue;
    vec3 normal = silhouette
      ? normalize(faceNormal[edge.z] + faceNormal[edge.w])
      : normalize(faceNormal[edge.z] + faceNormal[edge.w]);
    if (dot(normal, view) < 0.0) normal *= -1.0;
    float fresnel = pow(1.0 - clamp(abs(dot(normal, view)), 0.0, 1.0), 2.25);
    vec3 reflected = normalize(reflect(incident, normal));
    float white = pow(max(dot(reflected, normalize(vec3(-0.18, 0.86, 0.48))), 0.0), 14.0);
    vec2 start = projected[edge.x];
    vec2 end = projected[edge.y];
    float tangentBias = atan(end.y - start.y, end.x - start.x) / (2.0 * PI);
    float spectralEnergy;
    float hue;
    vec3 spectralColor = spinnerSpectralEnvironment(
      reflected,
      1.458,
      0.18,
      tangentBias,
      true,
      spectralEnergy,
      hue
    );
    vec3 sheenColor = spinnerNormalizeLuma(
      spinnerSpectralSample(fract(spinnerPhase * 3.0 + float(edgeIndex) * 0.38196601125), 1.458, 0.18, true, true),
      dot(spectralColor, vec3(0.2126, 0.7152, 0.0722)) * 1.04
    );
    float adjacentSheen = max(faceSheen[edge.z], faceSheen[edge.w]);
    float hdrEnergy = clamp(
      white * (silhouette ? 0.78 : 0.68)
        + adjacentSheen * (silhouette ? 0.28 : 0.36)
        + fresnel * (silhouette ? 0.07 : 0.10),
      0.0,
      1.0
    );
    vec3 spinnerEdgeColor = mix(spectralColor, sheenColor, smoothstep(0.08, 0.45, hdrEnergy));
    spinnerEdgeColor = mix(
      spinnerEdgeColor,
      vec3(0.9882, 1.0, 1.0),
      clamp(white * 0.24 + fresnel * 0.08, 0.0, 1.0)
    );
    vec3 edgeColor = mix(
      bridgeSpectralEdgeColor(start, end),
      spinnerEdgeColor,
      spinnerEdgeStyleDevelopment(edgeDevelopment)
    );
    float strokeWidthCss = silhouette ? 0.80 : 0.54;
    float halfWidth = strokeWidthCss * cssPixelWidth * 0.5;
    float distanceValue = spinnerSegmentDistance(point, start, end);
    float coverage = 1.0 - smoothstep(
      max(halfWidth - cssPixelWidth * 0.5, 0.0),
      halfWidth + cssPixelWidth * 0.5,
      distanceValue
    );
    float targetAlpha = silhouette ? 0.94 : 0.68;
    float topologyDevelopment = silhouette
      ? edgeDevelopment
      : spinnerCreaseActivation(edgeIndex, bestCreaseIndex, creaseDevelopment);
    float sourceAlpha = coverage * targetAlpha * topologyDevelopment;
    result.rgb = edgeColor * sourceAlpha + result.rgb * (1.0 - sourceAlpha);
    result.a = sourceAlpha + result.a * (1.0 - sourceAlpha);
    float hdrPresence = coverage
      * smoothstep(0.08, 0.62, hdrEnergy)
      * topologyDevelopment;
    hdrRadiance = edgeColor * hdrPresence + hdrRadiance * (1.0 - hdrPresence);
  }
  return result;
}

vec2 projectWorldToView(vec3 point, vec3 cameraOrigin) {
  float depth = max(cameraOrigin.z - point.z, 0.05);
  return (point.xy - cameraOrigin.xy) * (2.96 / depth);
}

vec3 lightPositionForIndex(
  int lightIndex,
  mat3 objectToWorld,
  float relativeLightAngle
) {
  float count = float(max(uLightCount, 1));
  float angularOffset = float(lightIndex) * 2.0 * PI / count;
  return objectToWorld
    * rotateZ(relativeLightAngle + angularOffset)
    * vec3(uLightRadius * renderPrismScale(), 0.0, 0.0);
}

void overLightLayer(
  vec3 color,
  float layerAlpha,
  inout vec3 accumulatedPremultiplied,
  inout float accumulatedAlpha
) {
  float contribution = clamp(layerAlpha, 0.0, 0.92) * (1.0 - accumulatedAlpha);
  accumulatedPremultiplied += color * contribution;
  accumulatedAlpha += contribution;
}

bool spectralFanBoundsReject(
  vec2 sampleCoordinate,
  vec2 boundsMinimum,
  vec2 boundsMaximum,
  float margin
) {
  vec2 paddedMinimum = boundsMinimum - vec2(margin);
  vec2 paddedMaximum = boundsMaximum + vec2(margin);
  return any(lessThan(sampleCoordinate, paddedMinimum))
    || any(greaterThan(sampleCoordinate, paddedMaximum));
}

void continuousSpectralFan(
  vec3 cameraOrigin,
  vec3 cameraDirection,
  vec2 sampleCoordinate,
  vec2 sampleCoordinateDx,
  vec2 sampleCoordinateDy,
  int lightIndex,
  int sampleIndex,
  out vec3 fanColor,
  out float fanAlpha,
  out vec3 fanRadiance,
  out vec3 incidentEntry,
  out float hasIncidentEntry
) {
  vec3 exits[SPECTRAL_ANCHOR_COUNT];
  vec3 directions[SPECTRAL_ANCHOR_COUNT];
  vec2 projectedFarPoints[SPECTRAL_ANCHOR_COUNT];
  float valid[SPECTRAL_ANCHOR_COUNT];
  incidentEntry = vec3(0.0);
  hasIncidentEntry = 0.0;

  for (int anchorIndex = 0; anchorIndex < SPECTRAL_ANCHOR_COUNT; anchorIndex += 1) {
    vec3 entryPoint;
    vec3 exitPoint;
    vec3 outgoingDirection;
    bool escaped = loadOpticalLightPath(
      lightIndex,
      sampleIndex,
      anchorIndex,
      entryPoint,
      exitPoint,
      outgoingDirection
    );
    valid[anchorIndex] = escaped ? 1.0 : 0.0;
    exits[anchorIndex] = exitPoint;
    directions[anchorIndex] = outgoingDirection;
    if (escaped && hasIncidentEntry < 0.5) {
      incidentEntry = entryPoint;
      hasIncidentEntry = 1.0;
    }
  }

  // The fan volume ends only after several physical attenuation lengths.
  // Keeping a fixed world-space span while the prism shrinks leaves faint HDR
  // haze at the canvas edge and exposes the rectangular render boundary.
  float outgoingLength = max(uSpectralFanReach, 0.02);
  float referenceLength = clamp(
    uScaledScatteringFalloff * 2.0,
    0.28 * renderPrismScale(),
    outgoingLength
  );
  vec2 commonOrigin = vec2(0.0);
  float commonOriginWeight = 0.0;
  vec2 fanBoundsMinimum = vec2(1e6);
  vec2 fanBoundsMaximum = vec2(-1e6);
  for (int anchorIndex = 0; anchorIndex < SPECTRAL_ANCHOR_COUNT; anchorIndex += 1) {
    if (valid[anchorIndex] < 0.5) continue;
    vec2 projectedExit = projectWorldToView(exits[anchorIndex], cameraOrigin);
    vec2 projectedFar = projectWorldToView(
      exits[anchorIndex] + directions[anchorIndex] * outgoingLength,
      cameraOrigin
    );
    projectedFarPoints[anchorIndex] = projectedFar;
    commonOrigin += projectedExit;
    commonOriginWeight += 1.0;
    fanBoundsMinimum = min(fanBoundsMinimum, min(projectedExit, projectedFar));
    fanBoundsMaximum = max(fanBoundsMaximum, max(projectedExit, projectedFar));
  }
  commonOrigin /= max(commonOriginWeight, 1.0);

  // Most low-resolution light pixels are outside a given source sample's fan.
  // Reject them before reconstructing six wavelength intervals and volumetric
  // lobes. The generous padding includes area-source feather, caustic width,
  // turbulence, and one projected fan span, so this changes no visible field.
  vec2 fanBoundsExtent = max(fanBoundsMaximum - fanBoundsMinimum, vec2(0.0));
  float fanBoundsMargin = (
    0.050
    + uSourceSize * 0.22
    + uSourceDivergence * 0.05
    + uBeamWidth * 0.018
  ) * renderPrismScale() + length(fanBoundsExtent) * 0.55;
  if (commonOriginWeight < 0.5 || spectralFanBoundsReject(
    sampleCoordinate,
    fanBoundsMinimum,
    fanBoundsMaximum,
    fanBoundsMargin
  )) {
    fanColor = vec3(0.0);
    fanAlpha = 0.0;
    fanRadiance = vec3(0.0);
    return;
  }

  vec2 referenceCenter = vec2(0.0);
  for (int anchorIndex = 0; anchorIndex < SPECTRAL_ANCHOR_COUNT; anchorIndex += 1) {
    if (valid[anchorIndex] < 0.5) continue;
    referenceCenter += projectWorldToView(
      exits[anchorIndex] + directions[anchorIndex] * referenceLength,
      cameraOrigin
    );
  }
  referenceCenter /= max(commonOriginWeight, 1.0);
  float bestEnergy = 0.0;
  vec3 bestColor = vec3(0.0);
  float totalSpread = 0.0;
  vec2 blueReference = referenceCenter;
  vec2 redReference = referenceCenter;
  if (valid[0] > 0.5 && valid[SPECTRAL_ANCHOR_COUNT - 1] > 0.5) {
    blueReference = projectWorldToView(exits[0] + directions[0] * referenceLength, cameraOrigin);
    redReference = projectWorldToView(
      exits[SPECTRAL_ANCHOR_COUNT - 1] + directions[SPECTRAL_ANCHOR_COUNT - 1] * referenceLength,
      cameraOrigin
    );
    totalSpread = length(redReference - blueReference);
  }

  for (int intervalIndex = 0; intervalIndex < SPECTRAL_ANCHOR_COUNT - 1; intervalIndex += 1) {
    if (valid[intervalIndex] < 0.5 || valid[intervalIndex + 1] < 0.5) continue;
    vec2 farFirst = projectedFarPoints[intervalIndex];
    vec2 farSecond = projectedFarPoints[intervalIndex + 1];
    vec2 firstEdge = farFirst - commonOrigin;
    vec2 secondEdge = farSecond - commonOrigin;
    float area = cross2D(firstEdge, secondEdge);
    if (abs(area) < 1e-6) continue;

    vec2 relative = sampleCoordinate - commonOrigin;
    float firstWeight = cross2D(relative, secondEdge) / area;
    float secondWeight = cross2D(firstEdge, relative) / area;
    float radialWeight = firstWeight + secondWeight;
    float oppositeWeight = 1.0 - radialWeight;
    float edgeDistance = min(min(firstWeight, secondWeight), oppositeWeight);
    vec2 firstWeightGradient = vec2(secondEdge.y, -secondEdge.x) / area;
    vec2 secondWeightGradient = vec2(-firstEdge.y, firstEdge.x) / area;
    vec2 edgeGradient = firstWeightGradient;
    if (secondWeight <= firstWeight && secondWeight <= oppositeWeight) {
      edgeGradient = secondWeightGradient;
    } else if (oppositeWeight <= firstWeight && oppositeWeight <= secondWeight) {
      edgeGradient = -(firstWeightGradient + secondWeightGradient);
    }
    float edgePixelWidth = abs(dot(edgeGradient, sampleCoordinateDx))
      + abs(dot(edgeGradient, sampleCoordinateDy));
    float areaSourceFeather = (uSourceSize * 0.034 + uSourceDivergence * 0.006) * renderPrismScale();
    float feather = max(
      edgePixelWidth * (0.85 + uBeamWidth * 0.45) + areaSourceFeather,
      0.0015
    );
    float coverage = smoothstep(-feather, feather, edgeDistance);
    float wavelengthMix = clamp(secondWeight / max(radialWeight, 0.0001), 0.0, 1.0);
    float wavelength = mix(
      wavelengthForSample(intervalIndex),
      wavelengthForSample(intervalIndex + 1),
      wavelengthMix
    );
    float distanceFromPrism = clamp(radialWeight, 0.0, 1.0) * outgoingLength;
    float atmosphericClarity = exp(-distanceFromPrism / max(uScaledScatteringFalloff, 0.02));
    float nearField = smoothstep(0.0, 0.035, max(radialWeight, 0.0));
    float energy = coverage * atmosphericClarity * nearField * uScatteringStrength * 0.86;
    if (energy > bestEnergy) {
      bestEnergy = energy;
      bestColor = spectrumColorForWavelength(wavelength);
    }
  }

  float causticHaloEnergy = 0.0;
  vec3 causticHaloColor = scatteringTint();
  vec2 fanAxis = referenceCenter - commonOrigin;
  float fanAxisLength = length(fanAxis);
  if (uCausticHalo > 0.0001 && fanAxisLength > 0.0001 && commonOriginWeight > 0.5) {
    vec2 alongDirection = fanAxis / fanAxisLength;
    vec2 acrossDirection = vec2(-alongDirection.y, alongDirection.x);
    if (dot(acrossDirection, redReference - blueReference) < 0.0) acrossDirection *= -1.0;
    vec2 haloRelative = sampleCoordinate - commonOrigin;
    float longitudinal = dot(haloRelative, alongDirection);
    float lateral = dot(haloRelative, acrossDirection);
    float haloLength = max(fanAxisLength * 1.75, 0.12 * renderPrismScale());
    float progress = clamp(longitudinal / max(haloLength, 0.001), 0.0, 1.0);
    float haloWidth = (0.018 + uSourceSize * 0.052) * renderPrismScale()
      + totalSpread * (0.42 + progress * 0.56);
    float forwardGate = smoothstep(-0.012 * renderPrismScale(), 0.026 * renderPrismScale(), longitudinal);
    float normalizedLateral = lateral / max(haloWidth, 0.002);
    float lateralFalloff = exp(-normalizedLateral * normalizedLateral * 1.32);
    float distanceFalloff = exp(-max(longitudinal, 0.0) / max(haloLength * 0.82, 0.02));
    float wavelengthMix = clamp(0.5 + lateral / max(haloWidth * 2.0, 0.004), 0.0, 1.0);
    causticHaloColor = spectrumColorForWavelength(mix(440.0, 650.0, wavelengthMix));
    float reachGate = 1.0 - smoothstep(
      outgoingLength * 0.82,
      outgoingLength,
      max(longitudinal, 0.0)
    );
    causticHaloEnergy = forwardGate
      * lateralFalloff
      * distanceFalloff
      * reachGate
      * uCausticHalo
      * uScatteringStrength
      * 0.24;
  }

  float whiteEnergy = 0.0;
  if (valid[3] > 0.5) {
    vec2 whiteVolume = beamSingleScattering(
      cameraOrigin,
      cameraDirection,
      100.0,
      exits[3],
      directions[3],
      outgoingLength,
      0.0,
      1.0,
      uScatteringStrength * (2.2 + uSourceHalo * 0.48),
      0.030 + uSourceSize * 0.18,
      0.018 + uSourceSize * 0.10 + uSourceDivergence * 0.026
    );
    whiteEnergy = whiteVolume.x + whiteVolume.y;
  }
  // Non-dispersed white volume is atmospheric fill, not the spectral beam. It
  // remains almost absent on dark surfaces and is capped on bright ones so a
  // light background cannot turn the surrounding air into an emissive cloud.
  whiteEnergy *= mix(0.001, 0.08, environmentLightMix());

  float fanMix = smoothstep(0.004, 0.028, totalSpread);
  // The fan represents a fixed amount of transmitted light. As dispersion
  // and the finite source make its footprint wider, that energy is distributed
  // across more screen area instead of turning the entire footprint into one
  // equally bright HDR slab. Keep the SDR haze styling below, but expose this
  // power-conserving radiance to the HDR path.
  float fanReferenceWidth = 0.032 * renderPrismScale();
  float fanFootprint = totalSpread
    + (uSourceSize * 0.034 + uSourceDivergence * 0.006) * renderPrismScale();
  float fanEnergyConservation = clamp(
    fanReferenceWidth / max(fanFootprint, fanReferenceWidth),
    0.72,
    1.0
  );
  float spectralPeak = max(max(bestColor.r, bestColor.g), bestColor.b);
  vec3 normalizedFanColor = spectralPeak > 1e-5 ? bestColor / spectralPeak : vec3(1.0);
  float baseEnergy = mix(whiteEnergy, bestEnergy, fanMix);
  vec3 baseColor = mix(scatteringTint(), normalizedFanColor, fanMix);
  float opticalDepth = baseEnergy + causticHaloEnergy;
  fanColor = opticalDepth > 1e-5
    ? (baseColor * baseEnergy + causticHaloColor * causticHaloEnergy) / opticalDepth
    : scatteringTint();
  float directSpectralDepth = bestEnergy * fanMix;
  float atmosphericFanDepth = max(opticalDepth - directSpectralDepth, 0.0);
  float visibleOpticalDepth = (
    directSpectralDepth * DIRECT_SPECTRAL_COVERAGE_GAIN
    + atmosphericFanDepth
  ) * 1.12;
  float transmittedLightEnergy = clamp(uLightThroughput, 0.0, 1.0);
  fanAlpha = clamp(1.0 - exp(-visibleOpticalDepth * transmittedLightEnergy), 0.0, 0.52)
    * uVisibility;
  // The high-luminance path belongs to the separated spectrum. White volume
  // and the soft caustic halo remain useful SDR atmosphere, but promoting them
  // with the same gain creates angle-dependent HDR clouds around the edges.
  vec3 directSpectralEnergy = normalizedFanColor * bestEnergy * fanMix;
  vec3 atmosphericFanEnergy = scatteringTint() * whiteEnergy * (1.0 - fanMix)
    + causticHaloColor * causticHaloEnergy;
  fanRadiance = (
    directSpectralEnergy
    + atmosphericFanEnergy * HDR_FAN_AIR_RELATIVE_GAIN
  ) * fanEnergyConservation
    * transmittedLightEnergy
    * uVisibility;
}

void main() {
  outGain = vec4(0.0);
  vec2 screenCoordinates = (gl_FragCoord.xy * 2.0 - uResolution) / uResolution.y * uViewportScale;
  float spinnerSilhouetteAlignment = uSpinnerMorph.x;
  // One physical prism owns the icon route and the settled material.
  bool iconMorphActive = uObjectProjection.w > 0.5;
  vec2 coordinates = screenCoordinates - vec2(0.0, iconMorphActive ? uObjectProjection.z : 0.0);
  vec2 spectralFanCoordinate = coordinates + refractiveTurbulence(coordinates);
  vec2 spectralFanCoordinateDx = dFdx(spectralFanCoordinate);
  vec2 spectralFanCoordinateDy = dFdy(spectralFanCoordinate);
  float edgeWidthCssPixels = uTransitionGeometry.y;
  float edgeHighlightProgress = uTransitionGeometry.z;
  float surfaceReveal = uTransitionGeometry.w;
  float surfaceTransition = uTransitionAppearance.x;
  float opticalTransition = uTransitionAppearance.y;
  float spectrumPassProgress = uTransitionAppearance.z;
  float iconEdgeAlpha = uTransitionAppearance.w;
  float spinnerCamera = uSpinnerMorph.y;
  float spinnerMaterial = uSpinnerMorph.w;
  float spinnerEdgeDevelopment = uSpinnerMaterialMorph.x;
  float spinnerSheenDevelopment = uSpinnerMaterialMorph.y;
  vec3 flatIconColor = iconMorphActive
    ? uIconEdgeColor.rgb
    : mix(vec3(0.1255, 0.1333, 0.1569), vec3(0.94, 0.94, 0.96), uDarkMode);
  float spinnerCameraDistance = 7.10 * 0.98 * transitionPrismScale();
  float prismCameraDistance = mix(4.20, 7.10, spinnerCamera);
  float prismProjectionDistance = mix(2.96, 7.10, spinnerCamera);
  float cameraDistance = iconMorphActive
    ? uObjectProjection.x
    : mix(prismCameraDistance, spinnerCameraDistance, spinnerCamera);
  float projectionDistance = iconMorphActive
    ? uObjectProjection.y
    : mix(prismProjectionDistance, spinnerCameraDistance, spinnerCamera);
  vec3 perspectiveOrigin = vec3(0.0, 0.0, cameraDistance);
  vec3 perspectiveDirection = normalize(vec3(coordinates.x, coordinates.y, -projectionDistance));
  vec3 rayOrigin = perspectiveOrigin;
  vec3 rayDirection = perspectiveDirection;

  float angle = uPhase * PI * 2.0;
  vec4 bridgeRotation;
  if (uObjectPoseOverride > 0.5) {
    // The ordinary prism pose and trajectory-owned morph pose are invariant
    // across the frame. Hosts resolve them once instead of repeating all
    // quaternion trigonometry for every light and material fragment.
    bridgeRotation = normalize(uObjectPoseQuaternion);
  } else {
    // Compatibility fallback for hosts that set morph progress directly
    // without supplying the trajectory pose.
    vec4 rotationYQuaternion = quaternionFromAxisAngle(vec3(0.0, 1.0, 0.0), angle);
    vec4 sourcePrismRotation = quaternionMultiply(
      quaternionMultiply(
        rotationYQuaternion,
        quaternionFromAxisAngle(vec3(1.0, 0.0, 0.0), -0.30)
      ),
      quaternionFromAxisAngle(vec3(0.0, 0.0, 1.0), 0.10)
    );
    vec4 spinnerRotation = quaternionMultiply(
      quaternionMultiply(
        quaternionFromAxisAngle(vec3(0.0, 0.0, 1.0), -0.03),
        quaternionFromAxisAngle(vec3(1.0, 0.0, 0.0), -0.11)
      ),
      quaternionFromAxisAngle(
        vec3(0.0, 1.0, 0.0),
        spinnerSourcePhase() * PI * 2.0 + 0.62
      )
    );
    float matchAngle = spinnerSourceWorldRotationPhase(uSpinnerMorphMatchPhase) * PI * 2.0;
    vec4 matchYRotation = quaternionFromAxisAngle(vec3(0.0, 1.0, 0.0), matchAngle);
    vec4 sourcePrismRotationAtMatch = quaternionMultiply(
      quaternionMultiply(
        matchYRotation,
        quaternionFromAxisAngle(vec3(1.0, 0.0, 0.0), -0.30)
      ),
      quaternionFromAxisAngle(vec3(0.0, 0.0, 1.0), 0.10)
    );
    vec4 matchedPrismRotation = quaternionMultiply(
      quaternionFromAxisAngle(vec3(0.0, 0.0, 1.0), PI * 0.25),
      matchYRotation
    );
    vec4 targetCorrection = quaternionMultiply(
      matchedPrismRotation,
      quaternionConjugate(sourcePrismRotationAtMatch)
    );
    vec4 correctedPrismRotation = quaternionMultiply(
      normalizedQuaternionMix(vec4(0.0, 0.0, 0.0, 1.0), targetCorrection, spinnerSilhouetteAlignment),
      sourcePrismRotation
    );
    bridgeRotation = normalizedQuaternionMix(
      correctedPrismRotation,
      spinnerRotation,
      spinnerMaterial
    );
  }
  mat3 objectToWorld = matrixFromQuaternion(bridgeRotation);
  mat3 worldToObject = transpose(objectToWorld);
  float relativeLightAngle = (uLightPhase - uPhase) * PI * 2.0 - 0.73;

  vec3 lightTarget = vec3(0.0);
  vec3 originLocal = worldToObject * rayOrigin;
  vec3 directionLocal = worldToObject * rayDirection;
  vec3 bridgeHdrRadiance = vec3(0.0);
  vec4 bridgeEdges = vec4(0.0);
  if (uRenderLayer == 1) {
    if (iconMorphActive) {
      float graphicEdgeWeight = clamp(iconEdgeAlpha, 0.0, 1.0);
      float physicalEdgeWeight = 1.0 - graphicEdgeWeight;
      float bevelResponse = clamp(surfaceReveal, 0.0, 1.0);
      vec3 iconBridgeHdrRadiance = vec3(0.0);
      vec4 iconBridgeEdges = prismIconEdgeSample(
        coordinates,
        objectToWorld,
        cameraDistance,
        projectionDistance,
        edgeWidthCssPixels,
        1.0,
        edgeHighlightProgress,
        iconBridgeHdrRadiance
      );
      vec3 settledBridgeHdrRadiance = vec3(0.0);
      vec4 settledBridgeEdges = bridgeEdgeSample(
        coordinates,
        objectToWorld,
        perspectiveOrigin,
        projectionDistance,
        settledBridgeHdrRadiance
      );
      vec4 graphicEdgeLayer = iconBridgeEdges * graphicEdgeWeight;
      vec4 physicalEdgeLayer = settledBridgeEdges * physicalEdgeWeight * bevelResponse;
      bridgeEdges = graphicEdgeLayer + physicalEdgeLayer * (1.0 - graphicEdgeLayer.a);
      bridgeHdrRadiance = iconBridgeHdrRadiance * graphicEdgeWeight
        + settledBridgeHdrRadiance * physicalEdgeWeight * bevelResponse;
    } else {
      bridgeEdges = bridgeEdgeSample(
        coordinates,
        objectToWorld,
        perspectiveOrigin,
        projectionDistance,
        bridgeHdrRadiance
      );
    }
  }

  if (iconMorphActive && surfaceTransition <= 0.0001) {
    outGain = encodeHdrGain(bridgeHdrRadiance, bridgeEdges.a);
    outColor = uRenderLayer == 1 ? bridgeEdges : vec4(0.0);
    return;
  }

  float nearDistance;
  float farDistance;
  vec3 nearPlaneNormal;
  vec3 farPlaneNormal;
  bool hit = intersectPrism(originLocal, directionLocal, nearDistance, farDistance, nearPlaneNormal, farPlaneNormal)
    && nearDistance > 0.0;

  if (uRenderLayer == 0 && hit) {
    outColor = vec4(0.0);
    return;
  }

  if (uRenderLayer == 1 && !hit) {
    outGain = encodeHdrGain(bridgeHdrRadiance, bridgeEdges.a);
    outColor = bridgeEdges;
    return;
  }

  if (uRenderLayer == 1 && uSpinnerMorph.z > 0.5 && spinnerMaterial > 0.999) {
    vec3 spinnerEntryPoint = originLocal + directionLocal * nearDistance;
    vec3 spinnerNormalWorld = normalize(objectToWorld * nearPlaneNormal);
    vec3 spinnerFaceColor;
    float spinnerFaceAlpha;
    vec3 spinnerFaceHdrRadiance;
    spinnerSurfaceMaterial(
      spinnerNormalWorld,
      min(nearestPrismPlaneIndex(spinnerEntryPoint), 7),
      spinnerSheenDevelopment,
      spinnerFaceColor,
      spinnerFaceAlpha,
      spinnerFaceHdrRadiance
    );
    float outputAlpha = spinnerFaceAlpha * surfaceTransition;
    vec3 outputColor = mix(flatIconColor, spinnerFaceColor, surfaceTransition);
    vec3 premultiplied = outputColor * outputAlpha;
    premultiplied = bridgeEdges.rgb + premultiplied * (1.0 - bridgeEdges.a);
    outputAlpha = bridgeEdges.a + outputAlpha * (1.0 - bridgeEdges.a);
    outGain = encodeHdrGain(spinnerFaceHdrRadiance + bridgeHdrRadiance, outputAlpha);
    outColor = vec4(premultiplied, outputAlpha);
    return;
  }

  if (!hit) {
    vec3 accumulatedPremultiplied = vec3(0.0);
    float accumulatedAlpha = 0.0;
    vec3 accumulatedHdrRadiance = vec3(0.0);
    float lightEnergyScale = inversesqrt(float(max(uLightCount, 1)));
    for (int lightIndex = 0; lightIndex < MAX_LIGHTS; lightIndex += 1) {
      if (lightIndex >= uLightCount) break;
      vec3 currentLightPosition = lightPositionForIndex(
        lightIndex,
        objectToWorld,
        relativeLightAngle
      );
      int sourceSampleCount = clamp(uSourceSampleCount, 1, MAX_SOURCE_SAMPLES);
      float sampleEnergyScale = 1.0 / float(sourceSampleCount);
      vec3 sampledFanPremultiplied = vec3(0.0);
      float sampledFanAlpha = 0.0;
      vec3 sampledFanRadiance = vec3(0.0);
      vec3 sampledIncidentPremultiplied = vec3(0.0);
      float sampledIncidentAlpha = 0.0;
      vec3 sampledIncidentRadiance = vec3(0.0);
      vec3 incidentWhiteTint = vec3(1.0);

      for (int sampleIndex = 0; sampleIndex < MAX_SOURCE_SAMPLES; sampleIndex += 1) {
        if (sampleIndex >= sourceSampleCount) break;
        vec3 sampleLightPosition;
        vec3 sampleLightTarget;
        areaSourceRay(
          currentLightPosition,
          lightTarget,
          sampleIndex,
          sourceSampleCount,
          sampleLightPosition,
          sampleLightTarget
        );

        vec3 spectralColor;
        float spectralAlpha;
        vec3 spectralRadiance;
        vec3 firstEntry;
        float foundEntry;
        continuousSpectralFan(
          rayOrigin,
          rayDirection,
          spectralFanCoordinate,
          spectralFanCoordinateDx,
          spectralFanCoordinateDy,
          lightIndex,
          sampleIndex,
          spectralColor,
          spectralAlpha,
          spectralRadiance,
          firstEntry,
          foundEntry
        );
        float weightedFanAlpha = spectralAlpha * sampleEnergyScale;
        sampledFanPremultiplied += spectralColor * weightedFanAlpha;
        sampledFanAlpha += weightedFanAlpha;
        sampledFanRadiance += spectralRadiance * sampleEnergyScale;

        if (foundEntry > 0.5) {
          vec3 incidentVector = firstEntry - sampleLightPosition;
          float incidentLength = length(incidentVector);
          float sourceRadius = 0.012 + uSourceSize * 0.16;
          float prismRadius = 0.020 + uSourceSize * 0.22 + uSourceDivergence * 0.075;
          vec2 incidentBody = beamSingleScattering(
            rayOrigin,
            rayDirection,
            100.0,
            sampleLightPosition,
            normalize(incidentVector),
            incidentLength,
            incidentLength,
            -1.0,
            uIncidentStrength * (0.55 + uSourceHalo * 0.28),
            sourceRadius,
            prismRadius
          );
          float incidentBodyDepth = incidentBody.x + incidentBody.y;
          float bodyAlpha = clamp(1.0 - exp(-incidentBodyDepth * 1.18), 0.0, 0.44)
            * uVisibility
            * sampleEnergyScale
            * incidentAirDisplayScale();
          sampledIncidentPremultiplied += scatteringTint() * bodyAlpha;
          sampledIncidentAlpha += bodyAlpha;
          sampledIncidentRadiance += scatteringTint()
            * incidentBodyDepth
            * uVisibility
            * sampleEnergyScale;

          // A narrow collimated source retains the bright core. Area-source
          // presets omit it: their offset ray bundles overlap into a volume
          // instead of leaving an unrelated laser down the centre.
          if (sourceSampleCount == 1) {
            vec2 incidentCore = beamSingleScattering(
              rayOrigin,
              rayDirection,
              100.0,
              sampleLightPosition,
              normalize(incidentVector),
              incidentLength,
              incidentLength,
              -1.0,
              uIncidentStrength,
              max(sourceRadius * 0.28, 0.008),
              max(prismRadius * 0.30, 0.012)
            );
            float incidentCoreDepth = incidentCore.x + incidentCore.y;
            float coreAlpha = clamp(1.0 - exp(-incidentCoreDepth * 1.35), 0.0, 0.36)
              * uVisibility;
            sampledIncidentPremultiplied += incidentWhiteTint * coreAlpha;
            sampledIncidentAlpha += coreAlpha;
            sampledIncidentRadiance += incidentWhiteTint
              * incidentCoreDepth
              * uVisibility;
          }
        }
      }

      if (sampledFanAlpha > 0.0001) {
        overLightLayer(
          sampledFanPremultiplied / sampledFanAlpha,
          sampledFanAlpha * lightEnergyScale,
          accumulatedPremultiplied,
          accumulatedAlpha
        );
      }
      if (sampledIncidentAlpha > 0.0001) {
        overLightLayer(
          sampledIncidentPremultiplied / sampledIncidentAlpha,
          sampledIncidentAlpha * lightEnergyScale,
          accumulatedPremultiplied,
          accumulatedAlpha
        );
      }
      accumulatedHdrRadiance += (
        sampledFanRadiance * HDR_SPECTRAL_FAN_GAIN
        + sampledIncidentRadiance * HDR_INCIDENT_AIR_GAIN
      ) * lightEnergyScale;

      // The emitter itself stays outside the composited image. Its position,
      // size and halo still drive incident/scattered light above, but the UI
      // treatment shows only light that reaches or leaves the prism.
    }
    vec3 accumulatedColor = accumulatedAlpha > 0.0001
      ? accumulatedPremultiplied / accumulatedAlpha
      : vec3(0.0);
    float outputAlpha = clamp(accumulatedAlpha, 0.0, 0.78)
      * opticalTransition
      * (1.0 - max(spinnerMaterial, uSpinnerMorph.z));
    float lightHdrPresence = opticalTransition * (1.0 - spinnerMaterial);
    outGain = encodeHdrGain(accumulatedHdrRadiance * lightHdrPresence, outputAlpha);
    outColor = vec4(clamp(accumulatedColor, 0.0, 1.0) * outputAlpha, outputAlpha);
    return;
  }

  float sizeCompensation = clamp(132.0 / max(min(uResolution.x, uResolution.y), 1.0), 1.0, 2.4);
  float dispersion = max(uDispersion, 0.0) * sizeCompensation;
  // The wavelength ordering follows crown glass. The angular separation is
  // deliberately magnified for a UI-sized object, but every channel still
  // traces the same closed geometry and fixed studio environment.
  float iorRed = iorForWavelength(650.0);
  float iorGreen = iorForWavelength(540.0);
  float iorBlue = iorForWavelength(420.0);

  float entryFresnelR;
  float exitFresnelR;
  float entryFresnelG;
  float exitFresnelG;
  float entryFresnelB;
  float exitFresnelB;
  vec3 entryPoint = originLocal + directionLocal * nearDistance;
  vec3 entryNormalLocal = polishedNormal(entryPoint, nearPlaneNormal);
  vec3 sharedReflectedEnvironment = roughEnvironment(
    objectToWorld * reflect(directionLocal, entryNormalLocal)
  );
  float red = traceChannelFromEntry(
    entryPoint,
    directionLocal,
    objectToWorld,
    entryNormalLocal,
    sharedReflectedEnvironment,
    iorRed,
    0,
    entryFresnelR,
    exitFresnelR
  );
  float green = traceChannelFromEntry(
    entryPoint,
    directionLocal,
    objectToWorld,
    entryNormalLocal,
    sharedReflectedEnvironment,
    iorGreen,
    1,
    entryFresnelG,
    exitFresnelG
  );
  float blue = traceChannelFromEntry(
    entryPoint,
    directionLocal,
    objectToWorld,
    entryNormalLocal,
    sharedReflectedEnvironment,
    iorBlue,
    2,
    entryFresnelB,
    exitFresnelB
  );
  vec3 color = vec3(red, green, blue);

  vec3 entryNormalWorld = normalize(objectToWorld * entryNormalLocal);
  vec3 entryPointWorld = objectToWorld * entryPoint;
  float entryPlaneConstant = 0.0;
  for (int planeIndex = 0; planeIndex < MAX_PRISM_PLANES; planeIndex += 1) {
    if (planeIndex >= uPrismPlaneCount) break;
    if (dot(uPrismPlanes[planeIndex].xyz, nearPlaneNormal) > 0.99999) {
      entryPlaneConstant = uPrismPlanes[planeIndex].w * transitionPrismScale();
      break;
    }
  }
  vec3 flatNormalWorld = normalize(objectToWorld * nearPlaneNormal);
  vec3 faceCenterWorld = objectToWorld * (nearPlaneNormal * entryPlaneConstant);

  // Trace the viewing ray after it enters the glass, then integrate only the
  // wavelength-dependent illumination segments it actually crosses inside.
  vec3 internalSpectralEnergy = vec3(0.0);
  vec3 behindSpectralEnergy = vec3(0.0);
  float internalLightScale = inversesqrt(float(max(uLightCount, 1)));
  int materialSampleCount = clamp(uMaterialSpectralSampleCount, 3, SPECTRAL_ANCHOR_COUNT);
  for (int materialSampleIndex = 0; materialSampleIndex < SPECTRAL_ANCHOR_COUNT; materialSampleIndex += 1) {
    if (materialSampleIndex >= materialSampleCount) break;
    int sampleIndex = materialAnchorIndex(materialSampleIndex, materialSampleCount);
    float materialSampleWeight = materialAnchorWeight(materialSampleIndex, materialSampleCount);
    float wavelength = wavelengthForSample(sampleIndex);
    float wavelengthIor = iorForWavelength(wavelength);
    vec3 viewInsideLocal = refract(directionLocal, entryNormalLocal, 1.0 / wavelengthIor);
    if (dot(viewInsideLocal, viewInsideLocal) < 1e-6) continue;
    viewInsideLocal = normalize(viewInsideLocal);
    vec3 viewInsideOriginLocal = entryPoint + viewInsideLocal * (0.0015 * renderPrismScale());
    float viewNear;
    float viewFar;
    vec3 viewNearNormal;
    vec3 viewFarNormal;
    if (!intersectPrism(
      viewInsideOriginLocal,
      viewInsideLocal,
      viewNear,
      viewFar,
      viewNearNormal,
      viewFarNormal
    )) continue;

    vec3 transmittedViewOrigin;
    vec3 transmittedViewDirection;
    bool transmittedViewEscaped = traceTransmittedViewRayFromInside(
      viewInsideOriginLocal,
      viewInsideLocal,
      viewFar,
      viewFarNormal,
      objectToWorld,
      wavelengthIor,
      transmittedViewOrigin,
      transmittedViewDirection
    );
    vec3 viewInsideOriginWorld = objectToWorld * viewInsideOriginLocal;
    vec3 viewInsideDirectionWorld = normalize(objectToWorld * viewInsideLocal);
    for (int lightIndex = 0; lightIndex < MAX_LIGHTS; lightIndex += 1) {
      if (lightIndex >= uLightCount) break;
      vec3 lightExit;
      vec3 lightOutgoing;
      vec3 internalOriginA;
      vec3 internalDirectionA;
      float internalLengthA;
      vec3 internalOriginB;
      vec3 internalDirectionB;
      float internalLengthB;
      bool escaped = loadOpticalMaterialPath(
        lightIndex,
        sampleIndex,
        lightExit,
        lightOutgoing,
        internalOriginA,
        internalDirectionA,
        internalLengthA,
        internalOriginB,
        internalDirectionB,
        internalLengthB
      );
      if (!escaped) continue;

      if (transmittedViewEscaped) {
        vec2 behindVolume = beamSingleScattering(
          transmittedViewOrigin + transmittedViewDirection * 0.002,
          transmittedViewDirection,
          100.0,
          lightExit,
          lightOutgoing,
          3.2,
          0.0,
          1.0,
          uScatteringStrength * (2.2 + uSourceHalo * 0.48),
          0.030 + uSourceSize * 0.18,
          0.018 + uSourceSize * 0.10 + uSourceDivergence * 0.026
        );
        behindSpectralEnergy += colorForWavelength(wavelength)
          * (behindVolume.x + behindVolume.y)
          * internalLightScale
          * materialSampleWeight;
      }

      vec2 internalVolume = beamSingleScattering(
        viewInsideOriginWorld,
        viewInsideDirectionWorld,
        viewFar,
        internalOriginA,
        internalDirectionA,
        internalLengthA,
        0.0,
        0.0,
        uScatteringStrength * 4.5,
        0.012 + uSourceSize * 0.035,
        0.012 + uSourceSize * 0.035 + uSourceDivergence * 0.008
      );
      if (internalLengthB > 0.0) {
        internalVolume += beamSingleScattering(
          viewInsideOriginWorld,
          viewInsideDirectionWorld,
          viewFar,
          internalOriginB,
          internalDirectionB,
          internalLengthB,
          0.0,
          0.0,
          uScatteringStrength * 4.5,
          0.012 + uSourceSize * 0.035,
          0.012 + uSourceSize * 0.035 + uSourceDivergence * 0.008
        );
      }
      internalSpectralEnergy += colorForWavelength(wavelength)
        * (internalVolume.x + internalVolume.y)
        * internalLightScale
        * materialSampleWeight;
    }
  }
  // Preserve wavelength energy per channel. Peak-normalizing every fragment
  // promoted even a weak single-wavelength hit to a fully saturated surface
  // wash; component-wise exposure keeps dim spectral paths dim and lets equal
  // wavelength energy reconstruct toward white.
  float transmittedLightEnergy = clamp(uLightThroughput, 0.0, 1.0);
  internalSpectralEnergy *= transmittedLightEnergy;
  behindSpectralEnergy *= transmittedLightEnergy;
  vec3 internalSpectralRadiance = vec3(1.0) - exp(
    -max(internalSpectralEnergy, vec3(0.0)) * 24.0
  );
  float internalGlow = clamp(
    dot(internalSpectralRadiance, vec3(0.2126, 0.7152, 0.0722)),
    0.0,
    0.46
  ) * uVisibility;
  vec3 behindSpectralRadiance = vec3(1.0) - exp(
    -max(behindSpectralEnergy, vec3(0.0)) * 1.1
  );
  float behindGlow = clamp(
    dot(behindSpectralRadiance, vec3(0.2126, 0.7152, 0.0722)),
    0.0,
    0.34
  ) * uVisibility;

  vec3 viewDirection = normalize(-rayDirection);
  float sourceSoftness = clamp(uSourceSize * 1.8 + uSourceDivergence * 0.18, 0.0, 1.0);
  float specularExponent = mix(420.0, 74.0, clamp(uRoughness, 0.0, 1.0))
    * mix(1.0, 0.32, sourceSoftness);
  float surfaceSpecular = 0.0;
  float faceLightVisibility = 0.0;
  float surfaceLightScale = inversesqrt(float(max(uLightCount, 1)));
  for (int lightIndex = 0; lightIndex < MAX_LIGHTS; lightIndex += 1) {
    if (lightIndex >= uLightCount) break;
    vec3 currentLightPosition = lightPositionForIndex(
      lightIndex,
      objectToWorld,
      relativeLightAngle
    );
    vec3 currentLightDirection = normalize(currentLightPosition - entryPointWorld);
    vec3 halfVector = normalize(currentLightDirection + viewDirection);
    surfaceSpecular += pow(max(dot(entryNormalWorld, halfVector), 0.0), specularExponent)
      * surfaceLightScale;
    faceLightVisibility += abs(dot(flatNormalWorld, normalize(currentLightPosition - faceCenterWorld)))
      * surfaceLightScale;
  }
  float edge = edgeFactor(entryPoint);
  float averageFresnel = (entryFresnelR + entryFresnelG + entryFresnelB) / 3.0;
  vec3 faceViewDirection = normalize(rayOrigin - faceCenterWorld);
  float noV = clamp(abs(dot(flatNormalWorld, faceViewDirection)), 0.0, 1.0);
  float sheenWidth = clamp((uSheenWidth - 0.15) / 2.85, 0.0, 1.0);
  float sheenExponent = mix(13.0, 1.35, sheenWidth);
  float grazingProfile = pow(1.0 - noV, sheenExponent);
  float physicalFresnel = fresnelSchlick(noV, uIor);
  float sheenAmount = grazingProfile
    * mix(0.28, 1.0, physicalFresnel)
    * max(uSheenStrength, 0.0)
    * mix(0.48, 1.0, clamp(faceLightVisibility, 0.0, 1.0));
  float thinFilmPhase = (1.0 - noV) * 1.85
    + dot(flatNormalWorld, normalize(vec3(0.31, 0.57, 0.76))) * 0.72;
  vec3 interferenceTint = 0.56 + 0.44 * cos(
    2.0 * PI * (thinFilmPhase + vec3(0.0, 0.333333, 0.666667))
  );
  float sheenChroma = clamp(uSheenChroma * (0.22 + max(uDispersion, 0.0) * 0.12), 0.0, 1.0);
  vec3 sheenReflection = mix(vec3(0.96, 0.98, 1.0), interferenceTint * 1.12, sheenChroma);

  float neutralValue = dot(color, vec3(0.2126, 0.7152, 0.0722));
  float chromaGain = 1.0 + dispersion * (0.10 + edge * 1.65);
  color = mix(vec3(neutralValue), color, chromaGain);
  // A dielectric should leave most of the backdrop visible at every ambient
  // luminance. Environment brightness changes the reflected studio energy,
  // but never restores an opaque neutral veil; colored sheen and spectral
  // paths remain independent so the glass can stay legible without whitening.
  float neutralSurfaceScale = mix(0.14, 0.72, environmentLightMix());
  color *= mix(0.28, 0.72, environmentLightMix());
  color += vec3(1.0) * surfaceSpecular * 0.58 * neutralSurfaceScale;
  color += sheenReflection * sheenAmount * 0.62;
  color += vec3(0.98) * edge * (0.034 + averageFresnel * 0.12) * neutralSurfaceScale;
  float spectralVisibility = uVisibility;
  float darkSpectralScale = mix(1.28, 1.0, environmentLightMix());
  vec3 compressedSpectralContribution = (
    behindSpectralRadiance * 0.28
    + internalSpectralRadiance * 0.42
  ) * spectralVisibility * darkSpectralScale;
  float activationWhiteBand = 0.0;
  float exitSpectrumAmount = 0.0;
  if (iconMorphActive) {
    prismActivationPass(
      entryPoint,
      spectrumPassProgress,
      activationWhiteBand,
      exitSpectrumAmount
    );
  }
  float internalPathLength = max(farDistance - nearDistance, 0.0)
    / max(transitionPrismScale(), 0.000001);
  float activationWhiteAmount = activationWhiteBand
    * clamp(internalPathLength * 0.45, 0.0, 1.0)
    * opticalTransition;
  compressedSpectralContribution *= opticalTransition * (1.0 + exitSpectrumAmount * 0.22);
  vec3 activationWhiteSdr = vec3(1.0) * activationWhiteAmount * 0.18;
  color += compressedSpectralContribution + activationWhiteSdr;
  // SDR deliberately compresses the volume with 1-exp(-energy), while HDR
  // receives only the corresponding pre-compression optical energy. Neutral
  // material color stays in the SDR base; treating it as radiance makes every
  // glass pixel saturate and turns the prism into a self-lit bulb.
  vec3 linearSpectralContribution = (
    max(behindSpectralEnergy, vec3(0.0)) * (1.1 * 0.28)
    + max(internalSpectralEnergy, vec3(0.0)) * (24.0 * 0.42 * HDR_GLASS_VOLUME_GAIN)
  ) * spectralVisibility * darkSpectralScale;
  linearSpectralContribution *= opticalTransition * (1.0 + exitSpectrumAmount * 0.16);
  vec3 activationWhiteHdr = vec3(1.0) * activationWhiteAmount * 0.55;
  vec3 prismPixelRadiance = linearSpectralContribution + activationWhiteHdr + bridgeHdrRadiance;
  color = clamp(color, 0.0, 1.18);
  color = color / (vec3(0.88) + color * 0.30);

  // One dielectric surface owns every intermediate frame. These are material
  // coefficients on that surface, not a second renderer mixed over it.
  vec3 spinnerFaceColor = vec3(0.0);
  float spinnerFaceAlpha = 0.0;
  vec3 spinnerFaceHdrRadiance = vec3(0.0);
  if (spinnerMaterial > 0.0) {
    spinnerSurfaceMaterial(
      flatNormalWorld,
      min(nearestPrismPlaneIndex(entryPoint), 7),
      spinnerSheenDevelopment,
      spinnerFaceColor,
      spinnerFaceAlpha,
      spinnerFaceHdrRadiance
    );
  }
  color = mix(color, spinnerFaceColor, spinnerMaterial);

  float neutralAlpha = (
    0.17
    + averageFresnel * 0.34 * uFresnelStrength
    + edge * 0.24
    + surfaceSpecular * 0.16
    + max(1.0 - uTransmission, 0.0) * 0.10
  ) * mix(0.22, 0.58, environmentLightMix());
  float spectralAlpha = (
    + sheenAmount * 0.20
    + internalGlow * 0.60
    + behindGlow * 0.34
  );
  float alpha = uVisibility * (neutralAlpha * uMaterialOpacityScale + spectralAlpha);
  float glassAlpha = clamp(
    mix(clamp(alpha, 0.0, 0.82) * surfaceTransition, spinnerFaceAlpha, spinnerMaterial),
    0.0,
    0.82
  );
  float morphOutline = morphSilhouette(entryPoint);
  float darkOutlineAlpha = iconMorphActive ? 0.0 : morphOutline * (1.0 - surfaceTransition);
  float outputAlpha = max(glassAlpha, darkOutlineAlpha);
  vec3 outputColor = mix(flatIconColor, clamp(color, 0.0, 1.0), surfaceTransition);
  vec3 premultiplied = outputColor * outputAlpha;
  premultiplied = bridgeEdges.rgb + premultiplied * (1.0 - bridgeEdges.a);
  outputAlpha = bridgeEdges.a + outputAlpha * (1.0 - bridgeEdges.a);
  vec3 spinnerPixelRadiance = spinnerFaceHdrRadiance + bridgeHdrRadiance;
  vec3 pixelRadiance = mix(prismPixelRadiance, spinnerPixelRadiance, spinnerMaterial);
  outGain = encodeHdrGain(pixelRadiance, outputAlpha);
  outColor = vec4(premultiplied, outputAlpha);
}
