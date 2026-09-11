export type Device = {
  width: number;
  height: number;
  radius: number;
  padding: number;
  paddingTop: number;
  fontSize: number;
  lineHeight: number;
  fontWeight: number;
  foot: number;
};

// 낱말이 하나씩 찍히는 창 — 마지막 낱말은 창 끝에서 시작해 TYPE_SPAN 뒤에 끝난다
export const TYPE_SPAN = 0.04;
export const TYPE_FIRST_RANGE = [0, 0.32] as const;
export const TYPE_SECOND_RANGE = [0.66, 0.97] as const;

export const LAPTOP: Device = {
  width: 960,
  height: 600,
  radius: 14,
  padding: 56,
  paddingTop: 88,
  fontSize: 40,
  lineHeight: 1.3,
  fontWeight: 600,
  foot: 1,
};

export const PHONE: Device = {
  width: 280,
  height: 608,
  radius: 44,
  padding: 22,
  paddingTop: 64,
  fontSize: 18,
  lineHeight: 1.45,
  fontWeight: 400,
  foot: 0,
};

export const mix = (from: number, to: number, t: number) => from + (to - from) * t;

export const lerpDevice = (from: Device, to: Device, t: number): Device => ({
  width: mix(from.width, to.width, t),
  height: mix(from.height, to.height, t),
  radius: mix(from.radius, to.radius, t),
  padding: mix(from.padding, to.padding, t),
  paddingTop: mix(from.paddingTop, to.paddingTop, t),
  fontSize: mix(from.fontSize, to.fontSize, t),
  lineHeight: mix(from.lineHeight, to.lineHeight, t),
  fontWeight: mix(from.fontWeight, to.fontWeight, t),
  foot: mix(from.foot, to.foot, t),
});
