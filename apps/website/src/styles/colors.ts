import { defineSemanticTokens, defineTokens } from '@pandacss/dev';

export const colors = defineTokens.colors({
  current: { value: 'currentColor' },

  white: { value: '#fff' },
  black: { value: '#000' },
  transparent: { value: 'rgb(0 0 0 / 0)' },

  brand: {
    '50': { value: 'oklch(0.962 0.018 272.314)' }, // #eef2ff
    '100': { value: 'oklch(0.93 0.034 272.788)' }, // #e0e7ff
    '200': { value: 'oklch(0.87 0.065 274.039)' }, // #c7d2fe
    '300': { value: 'oklch(0.785 0.115 274.713)' }, // #a5b4fc
    '400': { value: 'oklch(0.673 0.182 276.935)' }, // #818cf8
    '500': { value: 'oklch(0.585 0.233 277.117)' }, // #6366f1
    '600': { value: 'oklch(0.511 0.262 276.966)' }, // #4f46e5
    '700': { value: 'oklch(0.457 0.24 277.023)' }, // #4338ca
    '800': { value: 'oklch(0.398 0.195 277.366)' }, // #3730a3
    '900': { value: 'oklch(0.359 0.144 278.697)' }, // #312e81
    '950': { value: 'oklch(0.257 0.09 281.288)' }, // #1e1b4b
  },

  gray: {
    '50': { value: 'oklch(0.985 0 0)' }, // #fafafa
    '100': { value: 'oklch(0.967 0.001 286.375)' }, // #f4f4f5
    '200': { value: 'oklch(0.92 0.004 286.32)' }, // #e4e4e7
    '300': { value: 'oklch(0.871 0.006 286.286)' }, // #d4d4d8
    '400': { value: 'oklch(0.705 0.015 286.067)' }, // #a1a1aa
    '500': { value: 'oklch(0.552 0.016 285.938)' }, // #71717a
    '600': { value: 'oklch(0.442 0.017 285.786)' }, // #52525b
    '700': { value: 'oklch(0.37 0.013 285.805)' }, // #3f3f46
    '800': { value: 'oklch(0.274 0.006 286.033)' }, // #27272a
    '900': { value: 'oklch(0.21 0.006 285.885)' }, // #18181b
    '950': { value: 'oklch(0.141 0.005 285.823)' }, // #09090b
  },

  red: {
    '50': { value: 'oklch(0.971 0.013 17.38)' }, // #fef2f2
    '100': { value: 'oklch(0.936 0.032 17.717)' }, // #fee2e2
    '200': { value: 'oklch(0.885 0.062 18.334)' }, // #fecaca
    '300': { value: 'oklch(0.808 0.114 19.571)' }, // #fca5a5
    '400': { value: 'oklch(0.704 0.191 22.216)' }, // #f87171
    '500': { value: 'oklch(0.637 0.237 25.331)' }, // #ef4444
    '600': { value: 'oklch(0.577 0.245 27.325)' }, // #dc2626
    '700': { value: 'oklch(0.505 0.213 27.518)' }, // #b91c1c
    '800': { value: 'oklch(0.444 0.177 26.899)' }, // #991b1b
    '900': { value: 'oklch(0.396 0.141 25.723)' }, // #7f1d1d
    '950': { value: 'oklch(0.258 0.092 26.042)' }, // #450a0a
  },

  amber: {
    '50': { value: 'oklch(0.987 0.022 95.277)' }, // #fffbeb
    '100': { value: 'oklch(0.962 0.059 95.617)' }, // #fef3c7
    '200': { value: 'oklch(0.924 0.12 95.746)' }, // #fde68a
    '300': { value: 'oklch(0.879 0.169 91.605)' }, // #fcd34d
    '400': { value: 'oklch(0.828 0.189 84.429)' }, // #fbbf24
    '500': { value: 'oklch(0.769 0.188 70.08)' }, // #f59e0b
    '600': { value: 'oklch(0.666 0.179 58.318)' }, // #d97706
    '700': { value: 'oklch(0.555 0.163 48.998)' }, // #b45309
    '800': { value: 'oklch(0.473 0.137 46.201)' }, // #92400e
    '900': { value: 'oklch(0.414 0.112 45.904)' }, // #78350f
    '950': { value: 'oklch(0.279 0.077 45.635)' }, // #451a03
  },

  green: {
    '50': { value: 'oklch(0.982 0.018 155.826)' }, // #f0fdf4
    '100': { value: 'oklch(0.962 0.044 156.743)' }, // #dcfce7
    '200': { value: 'oklch(0.925 0.084 155.995)' }, // #bbf7d0
    '300': { value: 'oklch(0.871 0.15 154.449)' }, // #86efac
    '400': { value: 'oklch(0.792 0.209 151.711)' }, // #4ade80
    '500': { value: 'oklch(0.723 0.219 149.579)' }, // #22c55e
    '600': { value: 'oklch(0.627 0.194 149.214)' }, // #16a34a
    '700': { value: 'oklch(0.527 0.154 150.069)' }, // #15803d
    '800': { value: 'oklch(0.448 0.119 151.328)' }, // #166534
    '900': { value: 'oklch(0.393 0.095 152.535)' }, // #14532d
    '950': { value: 'oklch(0.266 0.065 152.934)' }, // #052e16
  },

  blue: {
    '50': { value: 'oklch(0.965 0.018 242.959)' }, // #eff6ff
    '100': { value: 'oklch(0.935 0.036 244.924)' }, // #dbeafe
    '200': { value: 'oklch(0.881 0.071 247.643)' }, // #bfdbfe
    '300': { value: 'oklch(0.803 0.124 250.325)' }, // #93c5fd
    '400': { value: 'oklch(0.713 0.186 252.694)' }, // #60a5fa
    '500': { value: 'oklch(0.636 0.232 254.791)' }, // #3b82f6
    '600': { value: 'oklch(0.571 0.247 256.614)' }, // #2563eb
    '700': { value: 'oklch(0.501 0.237 258.001)' }, // #1d4ed8
    '800': { value: 'oklch(0.431 0.199 258.882)' }, // #1e40af
    '900': { value: 'oklch(0.375 0.155 259.137)' }, // #1e3a8a
    '950': { value: 'oklch(0.258 0.102 260.256)' }, // #172554
  },

  dark: {
    gray: {
      '50': { value: 'oklch(0.92 0.002 266)' }, // #eaebeb
      '100': { value: 'oklch(0.87 0.002 266)' }, // #dbdcdc
      '200': { value: 'oklch(0.78 0.003 266)' }, // #c2c3c3
      '300': { value: 'oklch(0.67 0.003 266)' }, // #a4a5a5
      '400': { value: 'oklch(0.55 0.004 266)' }, // #838485
      '500': { value: 'oklch(0.45 0.004 266)' }, // #6a6b6c
      '600': { value: 'oklch(0.37 0.004 266)' }, // #565758
      '700': { value: 'oklch(0.31 0.004 266)' }, // #474849
      '800': { value: 'oklch(0.27 0.004 266)' }, // #3e3f40
      '900': { value: 'oklch(0.23 0.004 266)' }, // #353637
      '950': { value: 'oklch(0.19 0.004 266)' }, // #2c2d2e
    },

    brand: {
      '50': { value: 'oklch(0.85 0.08 276)' }, // #d2d4f0
      '100': { value: 'oklch(0.78 0.10 276)' }, // #bdc1ea
      '200': { value: 'oklch(0.70 0.13 276)' }, // #a1a8e1
      '300': { value: 'oklch(0.62 0.16 276)' }, // #828dd6
      '400': { value: 'oklch(0.55 0.18 276)' }, // #6270c9
      '500': { value: 'oklch(0.48 0.19 276)' }, // #4553ba
      '600': { value: 'oklch(0.42 0.18 276)' }, // #3a48a8
      '700': { value: 'oklch(0.36 0.16 276)' }, // #323d93
      '800': { value: 'oklch(0.30 0.12 276)' }, // #2a337b
      '900': { value: 'oklch(0.24 0.08 276)' }, // #232962
      '950': { value: 'oklch(0.19 0.06 276)' }, // #1c204a
    },

    red: {
      '50': { value: 'oklch(0.85 0.08 27)' }, // #f0d4d4
      '100': { value: 'oklch(0.78 0.10 27)' }, // #eac1c1
      '200': { value: 'oklch(0.70 0.13 27)' }, // #e1a8a8
      '300': { value: 'oklch(0.62 0.16 27)' }, // #d68d8d
      '400': { value: 'oklch(0.55 0.18 27)' }, // #c97070
      '500': { value: 'oklch(0.48 0.20 27)' }, // #ba5353
      '600': { value: 'oklch(0.42 0.19 27)' }, // #a84848
      '700': { value: 'oklch(0.36 0.17 27)' }, // #933d3d
      '800': { value: 'oklch(0.30 0.13 27)' }, // #7b3333
      '900': { value: 'oklch(0.24 0.09 27)' }, // #622929
      '950': { value: 'oklch(0.19 0.06 27)' }, // #4a2020
    },

    green: {
      '50': { value: 'oklch(0.85 0.08 152)' }, // #d4f0d4
      '100': { value: 'oklch(0.78 0.10 152)' }, // #c1eac1
      '200': { value: 'oklch(0.70 0.13 152)' }, // #a8e1a8
      '300': { value: 'oklch(0.62 0.16 152)' }, // #8dd68d
      '400': { value: 'oklch(0.55 0.18 152)' }, // #70c970
      '500': { value: 'oklch(0.48 0.19 152)' }, // #53ba53
      '600': { value: 'oklch(0.42 0.18 152)' }, // #48a848
      '700': { value: 'oklch(0.36 0.16 152)' }, // #3d933d
      '800': { value: 'oklch(0.30 0.12 152)' }, // #337b33
      '900': { value: 'oklch(0.24 0.08 152)' }, // #296229
      '950': { value: 'oklch(0.19 0.06 152)' }, // #204a20
    },

    blue: {
      '50': { value: 'oklch(0.85 0.08 256)' }, // #d4d8f0
      '100': { value: 'oklch(0.78 0.10 256)' }, // #c1c7ea
      '200': { value: 'oklch(0.70 0.13 256)' }, // #a8b0e1
      '300': { value: 'oklch(0.62 0.16 256)' }, // #8d96d6
      '400': { value: 'oklch(0.55 0.18 256)' }, // #707ac9
      '500': { value: 'oklch(0.48 0.19 256)' }, // #535eba
      '600': { value: 'oklch(0.42 0.18 256)' }, // #4850a8
      '700': { value: 'oklch(0.36 0.16 256)' }, // #3d4393
      '800': { value: 'oklch(0.30 0.12 256)' }, // #33377b
      '900': { value: 'oklch(0.24 0.08 256)' }, // #292c62
      '950': { value: 'oklch(0.19 0.06 256)' }, // #20224a
    },
  },
});

export const semanticColors = defineSemanticTokens.colors({
  'text.default': { value: { base: '{colors.gray.900}', _dark: '{colors.dark.gray.50}' } },
  'text.subtle': { value: { base: '{colors.gray.700}', _dark: '{colors.dark.gray.100}' } },
  'text.muted': { value: { base: '{colors.gray.600}', _dark: '{colors.dark.gray.200}' } },
  'text.faint': { value: { base: '{colors.gray.500}', _dark: '{colors.dark.gray.300}' } },
  'text.disabled': { value: { base: '{colors.gray.400}', _dark: '{colors.dark.gray.400}' } },
  'text.inverse': { value: { base: '{colors.white}', _dark: '{colors.dark.gray.900}' } },
  'text.danger': { value: { base: '{colors.red.500}', _dark: '{colors.dark.red.300}' } },
  'text.success': { value: { base: '{colors.green.500}', _dark: '{colors.dark.green.300}' } },
  'text.link': { value: { base: '{colors.blue.600}', _dark: '{colors.dark.blue.300}' } },
  'text.brand': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.300}' } },
  'text.overlay': { value: { base: '{colors.white}', _dark: '{colors.dark.gray.50}' } },
  'text.white': { value: { base: '{colors.white}', _dark: '{colors.white}' } },

  'surface.default': { value: { base: '{colors.white}', _dark: '{colors.dark.gray.900}' } },
  'surface.subtle': { value: { base: '{colors.gray.50}', _dark: '{colors.dark.gray.800}' } },
  'surface.muted': { value: { base: '{colors.gray.100}', _dark: '{colors.dark.gray.700}' } },
  'surface.overlay': { value: { base: '{colors.gray.700}', _dark: '{colors.dark.gray.700}' } },
  'surface.inverse': { value: { base: '{colors.gray.950}', _dark: '{colors.dark.gray.50}' } },

  'interactive.hover': { value: { base: '{colors.gray.200}', _dark: '{colors.dark.gray.600}' } },
  'interactive.disabled': { value: { base: '{colors.gray.200}', _dark: '{colors.dark.gray.800}' } },

  'accent.brand.default': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.400}' } },
  'accent.brand.hover': { value: { base: '{colors.brand.600}', _dark: '{colors.dark.brand.500}' } },
  'accent.brand.active': { value: { base: '{colors.brand.700}', _dark: '{colors.dark.brand.600}' } },
  'accent.brand.subtle': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.brand.900}' } },
  'accent.danger.default': { value: { base: '{colors.red.600}', _dark: '{colors.dark.red.400}' } },
  'accent.danger.hover': { value: { base: '{colors.red.500}', _dark: '{colors.dark.red.500}' } },
  'accent.danger.active': { value: { base: '{colors.red.700}', _dark: '{colors.dark.red.600}' } },
  'accent.danger.subtle': { value: { base: '{colors.red.50}', _dark: '{colors.dark.red.900}' } },
  'accent.success.subtle': { value: { base: '{colors.green.50}', _dark: '{colors.dark.green.900}' } },

  'border.default': { value: { base: '{colors.gray.200}', _dark: '{colors.dark.gray.700}' } },
  'border.strong': { value: { base: '{colors.gray.300}', _dark: '{colors.dark.gray.600}' } },
  'border.subtle': { value: { base: '{colors.gray.100}', _dark: '{colors.dark.gray.800}' } },

  'shadow.default': { value: { base: '{colors.gray.950}', _dark: 'oklch(0 0 0 / 0.5)' } },

  'control.scrollbar.default': { value: { base: '{colors.gray.200}', _dark: '{colors.dark.gray.600}' } },
  'control.scrollbar.hover': { value: { base: '{colors.gray.300}', _dark: '{colors.dark.gray.500}' } },

  'decoration.grid.default': { value: { base: '{colors.gray.100}', _dark: '{colors.dark.gray.800}' } },
  'decoration.grid.subtle': { value: { base: '{colors.gray.50}', _dark: '{colors.dark.gray.900}' } },
  'decoration.grid.brand': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.brand.900}' } },
  'decoration.grid.brand.subtle': { value: { base: '{colors.brand.50}', _dark: '{colors.dark.brand.950}' } },

  'callout.info': { value: { base: '#3b82f6', _dark: '#4c6ef5' } },
  'callout.success': { value: { base: '#22c55e', _dark: '#3fc380' } },
  'callout.warning': { value: { base: '#f97316', _dark: '#f4a934' } },
  'callout.danger': { value: { base: '#dc2626', _dark: '#f04444' } },
});
