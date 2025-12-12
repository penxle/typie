export const DPI = 96;

export function pxToCm(px: number, dpi = DPI): number {
  return (px / dpi) * 2.54;
}

export function cmToPx(cm: number, dpi = DPI): number {
  return (cm * dpi) / 2.54;
}

export type Tick = {
  position: number;
  isMajor: boolean;
  label?: string;
};

export function calculateTicks(totalSize: number, unit: 'px' | 'cm', dpi: number): Tick[] {
  const ticks: Tick[] = [];

  if (unit === 'px') {
    const majorInterval = 100;
    const minorInterval = 10;

    for (let pos = 0; pos <= totalSize; pos += minorInterval) {
      const isMajor = pos % majorInterval === 0;
      ticks.push({
        position: pos,
        isMajor,
        label: isMajor ? pos.toString() : undefined,
      });
    }
  } else if (unit === 'cm') {
    const minorIntervalCm = 0.25;
    const totalSizeCm = pxToCm(totalSize, dpi);
    const tickCount = Math.floor(totalSizeCm / minorIntervalCm);

    for (let i = 0; i <= tickCount; i++) {
      const cm = i * minorIntervalCm;
      const isMajor = i % 4 === 0;
      const position = cmToPx(cm, dpi);

      ticks.push({
        position,
        isMajor,
        label: isMajor ? (i / 4).toString() : undefined,
      });
    }
  }

  return ticks;
}

export function formatTickLabel(value: number, unit: 'px' | 'cm'): string {
  if (unit === 'px') {
    return value.toString();
  } else {
    return Math.round(value).toString();
  }
}
