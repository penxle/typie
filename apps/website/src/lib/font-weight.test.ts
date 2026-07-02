import { describe, expect, it } from 'vitest';
import { fontWeightItemsForFonts, fontWeightValueLabel, matchFontWeight, resolveFontWeightForFamily } from './font-weight';

const labels = [
  { value: 300, label: '가늘게' },
  { value: 900, label: '가장 굵게' },
];

describe('matchFontWeight', () => {
  it('uses CSS font-weight matching rather than absolute nearest', () => {
    expect(matchFontWeight([300, 600], 500)).toBe(300);
  });
});

describe('resolveFontWeightForFamily', () => {
  it('matches a heavy weight to the selected family weights', () => {
    expect(
      resolveFontWeightForFamily(
        [
          {
            familyName: 'LightFont',
            fonts: [
              { weight: 100, state: 'ACTIVE' },
              { weight: 300, state: 'ACTIVE' },
            ],
          },
        ],
        'LightFont',
        900,
      ),
    ).toBe(300);
  });

  it('keeps an unset weight unset', () => {
    expect(
      resolveFontWeightForFamily(
        [
          {
            familyName: 'LightFont',
            fonts: [{ weight: 300, state: 'ACTIVE' }],
          },
        ],
        'LightFont',
        undefined,
      ),
    ).toBeUndefined();
  });
});

describe('fontWeightValueLabel', () => {
  it('uses numeric fallback for unavailable weights', () => {
    expect(fontWeightValueLabel([{ weight: 300, state: 'ACTIVE' }], labels, 900)).toBe('900');
  });

  it('uses configured labels only for available weights', () => {
    expect(fontWeightValueLabel([{ weight: 900, state: 'ACTIVE' }], labels, 900)).toBe('가장 굵게');
  });
});

describe('fontWeightItemsForFonts', () => {
  it('uses only active family weights', () => {
    expect(
      fontWeightItemsForFonts(
        [
          { weight: 300, state: 'ACTIVE' },
          { weight: 900, state: 'INACTIVE' },
        ],
        labels,
      ),
    ).toEqual([{ value: 300, label: '가늘게' }]);
  });
});
