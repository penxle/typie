import { defineSemanticTokens, defineTokens } from '@pandacss/dev';

export const colors = defineTokens.colors({
  current: { value: 'currentColor' },

  white: { value: '#fff' },
  black: { value: '#000' },
  transparent: { value: 'rgb(0 0 0 / 0)' },

  brand: {
    '50': { value: 'oklch(0.962 0.018 272.314)' }, // #eef2ff
    '100': { value: 'oklch(0.93 0.034 272.788)' }, // #e0e7ff
    '200': { value: 'oklch(0.87 0.065 274.039)' }, // #c6d2ff
    '300': { value: 'oklch(0.785 0.115 274.713)' }, // #a3b3ff
    '400': { value: 'oklch(0.673 0.182 276.935)' }, // #7c86ff
    '500': { value: 'oklch(0.585 0.233 277.117)' }, // #615fff
    '600': { value: 'oklch(0.511 0.262 276.966)' }, // #4f39f6
    '700': { value: 'oklch(0.457 0.24 277.023)' }, // #432dd7
    '800': { value: 'oklch(0.398 0.195 277.366)' }, // #372aac
    '900': { value: 'oklch(0.359 0.144 278.697)' }, // #312c85
    '950': { value: 'oklch(0.257 0.09 281.288)' }, // #1e1a4d
  },

  gray: {
    '50': { value: 'oklch(0.985 0 0)' }, // #fafafa
    '100': { value: 'oklch(0.967 0.001 286.375)' }, // #f4f4f5
    '200': { value: 'oklch(0.92 0.004 286.32)' }, // #e4e4e7
    '300': { value: 'oklch(0.871 0.006 286.286)' }, // #d4d4d8
    '400': { value: 'oklch(0.705 0.015 286.067)' }, // #9f9fa9
    '500': { value: 'oklch(0.552 0.016 285.938)' }, // #71717b
    '600': { value: 'oklch(0.442 0.017 285.786)' }, // #52525c
    '700': { value: 'oklch(0.37 0.013 285.805)' }, // #3f3f46
    '800': { value: 'oklch(0.274 0.006 286.033)' }, // #27272a
    '900': { value: 'oklch(0.21 0.006 285.885)' }, // #18181b
    '950': { value: 'oklch(0.141 0.005 285.823)' }, // #09090b
  },

  red: {
    '50': { value: 'oklch(0.971 0.013 17.38)' }, // #fef2f2
    '100': { value: 'oklch(0.936 0.032 17.717)' }, // #ffe2e2
    '200': { value: 'oklch(0.885 0.062 18.334)' }, // #ffc9c9
    '300': { value: 'oklch(0.808 0.114 19.571)' }, // #ffa2a2
    '400': { value: 'oklch(0.704 0.191 22.216)' }, // #ff6467
    '500': { value: 'oklch(0.637 0.237 25.331)' }, // #fb2c36
    '600': { value: 'oklch(0.577 0.245 27.325)' }, // #e7000b
    '700': { value: 'oklch(0.505 0.213 27.518)' }, // #c10007
    '800': { value: 'oklch(0.444 0.177 26.899)' }, // #9f0712
    '900': { value: 'oklch(0.396 0.141 25.723)' }, // #82181a
    '950': { value: 'oklch(0.258 0.092 26.042)' }, // #460809
  },

  amber: {
    '50': { value: 'oklch(0.987 0.022 95.277)' }, // #fffbeb
    '100': { value: 'oklch(0.962 0.059 95.617)' }, // #fef3c6
    '200': { value: 'oklch(0.924 0.12 95.746)' }, // #fee685
    '300': { value: 'oklch(0.879 0.169 91.605)' }, // #ffd230
    '400': { value: 'oklch(0.828 0.189 84.429)' }, // #ffba00
    '500': { value: 'oklch(0.769 0.188 70.08)' }, // #fd9a00
    '600': { value: 'oklch(0.666 0.179 58.318)' }, // #e17100
    '700': { value: 'oklch(0.555 0.163 48.998)' }, // #bb4d00
    '800': { value: 'oklch(0.473 0.137 46.201)' }, // #973c00
    '900': { value: 'oklch(0.414 0.112 45.904)' }, // #7b3306
    '950': { value: 'oklch(0.279 0.077 45.635)' }, // #461901
  },

  green: {
    '50': { value: 'oklch(0.982 0.018 155.826)' }, // #f0fdf4
    '100': { value: 'oklch(0.962 0.044 156.743)' }, // #dcfce7
    '200': { value: 'oklch(0.925 0.084 155.995)' }, // #b9f8cf
    '300': { value: 'oklch(0.871 0.15 154.449)' }, // #7bf1a8
    '400': { value: 'oklch(0.792 0.209 151.711)' }, // #05df72
    '500': { value: 'oklch(0.723 0.219 149.579)' }, // #00c951
    '600': { value: 'oklch(0.627 0.194 149.214)' }, // #00a63e
    '700': { value: 'oklch(0.527 0.154 150.069)' }, // #008236
    '800': { value: 'oklch(0.448 0.119 151.328)' }, // #016630
    '900': { value: 'oklch(0.393 0.095 152.535)' }, // #0d542b
    '950': { value: 'oklch(0.266 0.065 152.934)' }, // #032e15
  },

  blue: {
    '50': { value: 'oklch(0.965 0.018 242.959)' }, // #eaf5ff
    '100': { value: 'oklch(0.935 0.036 244.924)' }, // #d6edff
    '200': { value: 'oklch(0.881 0.071 247.643)' }, // #b3ddff
    '300': { value: 'oklch(0.803 0.124 250.325)' }, // #7fc4ff
    '400': { value: 'oklch(0.713 0.186 252.694)' }, // #43a5ff
    '500': { value: 'oklch(0.636 0.232 254.791)' }, // #0087ff
    '600': { value: 'oklch(0.571 0.247 256.614)' }, // #006cff
    '700': { value: 'oklch(0.501 0.237 258.001)' }, // #0054e6
    '800': { value: 'oklch(0.431 0.199 258.882)' }, // #0044ba
    '900': { value: 'oklch(0.375 0.155 259.137)' }, // #003a91
    '950': { value: 'oklch(0.258 0.102 260.256)' }, // #021f53
  },

  dark: {
    gray: {
      '50': { value: 'oklch(0.92 0.002 266)' }, // #e4e4e6
      '100': { value: 'oklch(0.87 0.002 266)' }, // #d3d4d5
      '200': { value: 'oklch(0.78 0.003 266)' }, // #b6b7b9
      '300': { value: 'oklch(0.67 0.003 266)' }, // #949597
      '400': { value: 'oklch(0.55 0.004 266)' }, // #707174
      '500': { value: 'oklch(0.45 0.004 266)' }, // #545557
      '600': { value: 'oklch(0.37 0.004 266)' }, // #3f4042
      '700': { value: 'oklch(0.31 0.004 266)' }, // #2f3032
      '800': { value: 'oklch(0.27 0.004 266)' }, // #252628
      '900': { value: 'oklch(0.23 0.004 266)' }, // #1c1d1f
      '950': { value: 'oklch(0.19 0.004 266)' }, // #131416
    },

    brand: {
      '50': { value: 'oklch(0.85 0.08 276)' }, // #bfcaff
      '100': { value: 'oklch(0.78 0.10 276)' }, // #a6b2f7
      '200': { value: 'oklch(0.70 0.13 276)' }, // #8996ee
      '300': { value: 'oklch(0.62 0.16 276)' }, // #6e7ae5
      '400': { value: 'oklch(0.55 0.18 276)' }, // #5960d8
      '500': { value: 'oklch(0.48 0.19 276)' }, // #4748c5
      '600': { value: 'oklch(0.42 0.18 276)' }, // #3937ac
      '700': { value: 'oklch(0.36 0.16 276)' }, // #2c2a8e
      '800': { value: 'oklch(0.30 0.12 276)' }, // #212268
      '900': { value: 'oklch(0.24 0.08 276)' }, // #161944
      '950': { value: 'oklch(0.19 0.06 276)' }, // #0d102e
    },

    red: {
      '50': { value: 'oklch(0.85 0.08 27)' }, // #febab2
      '100': { value: 'oklch(0.78 0.10 27)' }, // #f19f95
      '200': { value: 'oklch(0.70 0.13 27)' }, // #e47c72
      '300': { value: 'oklch(0.62 0.16 27)' }, // #d5584f
      '400': { value: 'oklch(0.55 0.18 27)' }, // #c53732
      '500': { value: 'oklch(0.48 0.20 27)' }, // #b3000d
      '600': { value: 'oklch(0.42 0.19 27)' }, // #9b0000
      '700': { value: 'oklch(0.36 0.17 27)' }, // #7e0000
      '800': { value: 'oklch(0.30 0.13 27)' }, // #5f0001
      '900': { value: 'oklch(0.24 0.09 27)' }, // #400405
      '950': { value: 'oklch(0.19 0.06 27)' }, // #290605
    },

    green: {
      '50': { value: 'oklch(0.85 0.08 152)' }, // #a7ddb4
      '100': { value: 'oklch(0.78 0.10 152)' }, // #85ca98
      '200': { value: 'oklch(0.70 0.13 152)' }, // #58b575
      '300': { value: 'oklch(0.62 0.16 152)' }, // #0ea053
      '400': { value: 'oklch(0.55 0.18 152)' }, // #008c38
      '500': { value: 'oklch(0.48 0.19 152)' }, // #00752b
      '600': { value: 'oklch(0.42 0.18 152)' }, // #006121
      '700': { value: 'oklch(0.36 0.16 152)' }, // #004e17
      '800': { value: 'oklch(0.30 0.12 152)' }, // #003c0d
      '900': { value: 'oklch(0.24 0.08 152)' }, // #002909
      '950': { value: 'oklch(0.19 0.06 152)' }, // #001b05
    },

    blue: {
      '50': { value: 'oklch(0.85 0.08 256)' }, // #acd1ff
      '100': { value: 'oklch(0.78 0.10 256)' }, // #8dbaf7
      '200': { value: 'oklch(0.70 0.13 256)' }, // #66a0ee
      '300': { value: 'oklch(0.62 0.16 256)' }, // #3c86e4
      '400': { value: 'oklch(0.55 0.18 256)' }, // #106ed7
      '500': { value: 'oklch(0.48 0.19 256)' }, // #0057c5
      '600': { value: 'oklch(0.42 0.18 256)' }, // #0046ab
      '700': { value: 'oklch(0.36 0.16 256)' }, // #00368e
      '800': { value: 'oklch(0.30 0.12 256)' }, // #002a68
      '900': { value: 'oklch(0.24 0.08 256)' }, // #011e44
      '950': { value: 'oklch(0.19 0.06 256)' }, // #01132e
    },
  },
});

export const semanticColors = defineSemanticTokens.colors({
  'text.default': { value: { base: '{colors.gray.900}', _dark: '{colors.dark.gray.50}' } },
  'text.subtle': { value: { base: '{colors.gray.700}', _dark: '{colors.dark.gray.100}' } },
  'text.muted': { value: { base: '{colors.gray.600}', _dark: '{colors.dark.gray.200}' } },
  'text.faint': { value: { base: '{colors.gray.500}', _dark: '{colors.dark.gray.300}' } },
  'text.disabled': { value: { base: '{colors.gray.400}', _dark: '{colors.dark.gray.400}' } },
  'text.bright': { value: { base: '{colors.white}', _dark: '{colors.dark.gray.50}' } },
  'text.danger': { value: { base: '{colors.red.500}', _dark: '{colors.dark.red.300}' } },
  'text.success': { value: { base: '{colors.green.700}', _dark: '{colors.dark.green.300}' } },
  'text.link': { value: { base: '{colors.blue.600}', _dark: '{colors.dark.blue.400}' } },
  'text.brand': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.300}' } },

  'surface.default': { value: { base: '{colors.white}', _dark: '{colors.dark.gray.900}' } },
  'surface.subtle': { value: { base: '{colors.gray.50}', _dark: '{colors.dark.gray.800}' } },
  'surface.muted': { value: { base: '{colors.gray.100}', _dark: '{colors.dark.gray.700}' } },
  'surface.dark': { value: { base: '{colors.gray.700}', _dark: '{colors.dark.gray.700}' } },

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
  'border.brand': { value: { base: '{colors.brand.600}', _dark: '{colors.dark.brand.400}' } },
  'border.danger': { value: { base: '{colors.red.600}', _dark: '{colors.dark.red.400}' } },

  'shadow.default': { value: { base: '{colors.gray.950}', _dark: 'oklch(0 0 0 / 0.5)' } },

  'control.scrollbar.default': { value: { base: '{colors.gray.200}', _dark: '{colors.dark.gray.600}' } },
  'control.scrollbar.hover': { value: { base: '{colors.gray.300}', _dark: '{colors.dark.gray.500}' } },

  'decoration.grid.default': { value: { base: '{colors.gray.100}', _dark: '{colors.dark.gray.700}' } },
  'decoration.grid.subtle': { value: { base: '{colors.gray.50}', _dark: '{colors.dark.gray.800}' } },
  'decoration.grid.brand': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.gray.700}' } },
  'decoration.grid.brand.subtle': { value: { base: '{colors.brand.50}', _dark: '{colors.dark.gray.800}' } },

  'callout.info': { value: { base: '#3b82f6', _dark: '#4c6ef5' } },
  'callout.success': { value: { base: '#22c55e', _dark: '#3fc380' } },
  'callout.warning': { value: { base: '#f97316', _dark: '#f4a934' } },
  'callout.danger': { value: { base: '#dc2626', _dark: '#f04444' } },

  'prosemirror.black': { value: { base: '{colors.gray.900}', _dark: '{colors.dark.gray.50}' } },
  'prosemirror.gray': { value: { base: '#71717a' } },
  'prosemirror.white': { value: { base: '{colors.white}', _dark: '{colors.dark.gray.900}' } },
  'prosemirror.red': { value: { base: '#ef4444' } },
  'prosemirror.orange': { value: { base: '#f97316' } },
  'prosemirror.amber': { value: { base: '#f59e0b' } },
  'prosemirror.yellow': { value: { base: '#eab308' } },
  'prosemirror.lime': { value: { base: '#84cc16' } },
  'prosemirror.green': { value: { base: '#22c55e' } },
  'prosemirror.emerald': { value: { base: '#10b981' } },
  'prosemirror.teal': { value: { base: '#14b8a6' } },
  'prosemirror.cyan': { value: { base: '#06b6d4' } },
  'prosemirror.sky': { value: { base: '#0ea5e9' } },
  'prosemirror.blue': { value: { base: '#3b82f6' } },
  'prosemirror.indigo': { value: { base: '#6366f1' } },
  'prosemirror.violet': { value: { base: '#8b5cf6' } },
  'prosemirror.purple': { value: { base: '#a855f7' } },
  'prosemirror.fuchsia': { value: { base: '#d946ef' } },
  'prosemirror.pink': { value: { base: '#ec4899' } },
  'prosemirror.rose': { value: { base: '#f43f5e' } },

  'prosemirror.bg.gray': { value: { base: '#f1f1f2', _dark: '#38393b' } },
  'prosemirror.bg.red': { value: { base: '#fdebec', _dark: '#532f2b' } },
  'prosemirror.bg.orange': { value: { base: '#ffecd5', _dark: '#54341a' } },
  'prosemirror.bg.yellow': { value: { base: '#fef3c7', _dark: '#4e3e1b' } },
  'prosemirror.bg.green': { value: { base: '#dff3e3', _dark: '#2c4331' } },
  'prosemirror.bg.blue': { value: { base: '#e7f3f8', _dark: '#153b4f' } },
  'prosemirror.bg.purple': { value: { base: '#f0e7fe', _dark: '#3f2e50' } },
});
