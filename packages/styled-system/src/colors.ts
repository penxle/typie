import { defineSemanticTokens, defineTokens } from '@pandacss/dev';

export const colors = defineTokens.colors({
  current: { value: 'currentColor' },

  white: { value: '#fff' },
  black: { value: '#000' },
  transparent: { value: 'rgb(0 0 0 / 0)' },

  brand: {
    '50': { value: 'oklch(0.970 0.015 290)' }, // #f5f4ff
    '100': { value: 'oklch(0.935 0.032 288)' }, // #e8e7fe
    '200': { value: 'oklch(0.880 0.058 285)' }, // #d3d3fd
    '300': { value: 'oklch(0.800 0.092 283)' }, // #b5b7f8
    '400': { value: 'oklch(0.700 0.118 281)' }, // #9295e6
    '500': { value: 'oklch(0.580 0.135 280)' }, // #6c6fc8
    '600': { value: 'oklch(0.490 0.135 278)' }, // #5055ab
    '700': { value: 'oklch(0.410 0.125 276)' }, // #3a408c
    '800': { value: 'oklch(0.350 0.105 274)' }, // #2b3370
    '900': { value: 'oklch(0.290 0.082 272)' }, // #1e2753
    '950': { value: 'oklch(0.220 0.058 270)' }, // #101835
  },

  gray: {
    '50': { value: 'oklch(0.985 0.005 280)' }, // #f9fafd
    '100': { value: 'oklch(0.967 0.007 280)' }, // #f3f4f9
    '200': { value: 'oklch(0.920 0.010 280)' }, // #e3e4eb
    '300': { value: 'oklch(0.871 0.012 280)' }, // #d3d4dd
    '400': { value: 'oklch(0.705 0.014 280)' }, // #9e9fa9
    '500': { value: 'oklch(0.552 0.015 280)' }, // #70717b
    '600': { value: 'oklch(0.442 0.015 280)' }, // #51525b
    '700': { value: 'oklch(0.370 0.013 280)' }, // #3e3f47
    '800': { value: 'oklch(0.274 0.010 280)' }, // #26272c
    '900': { value: 'oklch(0.210 0.008 280)' }, // #17181c
    '950': { value: 'oklch(0.141 0.006 280)' }, // #09090c
  },

  red: {
    '50': { value: 'oklch(0.971 0.013 8)' }, // #fef2f4
    '100': { value: 'oklch(0.936 0.032 9)' }, // #fee2e5
    '200': { value: 'oklch(0.885 0.062 10)' }, // #ffc9d0
    '300': { value: 'oklch(0.808 0.110 11)' }, // #fea2af
    '400': { value: 'oklch(0.704 0.180 12)' }, // #fa6781
    '500': { value: 'oklch(0.637 0.220 13)' }, // #f23864
    '600': { value: 'oklch(0.577 0.230 14)' }, // #e0024e
    '700': { value: 'oklch(0.505 0.200 14)' }, // #bb0440
    '800': { value: 'oklch(0.444 0.170 15)' }, // #9b0d34
    '900': { value: 'oklch(0.396 0.138 15)' }, // #80182e
    '950': { value: 'oklch(0.258 0.090 16)' }, // #450814
  },

  amber: {
    '50': { value: 'oklch(0.987 0.022 85)' }, // #fffaea
    '100': { value: 'oklch(0.962 0.055 82)' }, // #fff0c9
    '200': { value: 'oklch(0.924 0.105 80)' }, // #ffdf94
    '300': { value: 'oklch(0.879 0.155 77)' }, // #ffc850
    '400': { value: 'oklch(0.828 0.170 74)' }, // #ffb31d
    '500': { value: 'oklch(0.769 0.170 70)' }, // #f79d00
    '600': { value: 'oklch(0.666 0.160 63)' }, // #d77a00
    '700': { value: 'oklch(0.555 0.140 57)' }, // #ae5900
    '800': { value: 'oklch(0.473 0.120 54)' }, // #8d4500
    '900': { value: 'oklch(0.414 0.100 52)' }, // #74390a
    '950': { value: 'oklch(0.279 0.070 50)' }, // #431c03
  },

  green: {
    '50': { value: 'oklch(0.986 0.021 160)' }, // #effff6
    '100': { value: 'oklch(0.968 0.048 160)' }, // #daffea
    '200': { value: 'oklch(0.921 0.076 160)' }, // #b9f5d4
    '300': { value: 'oklch(0.851 0.130 160)' }, // #79e8b1
    '400': { value: 'oklch(0.754 0.185 160)' }, // #00d185
    '500': { value: 'oklch(0.640 0.160 162)' }, // #00a96d
    '600': { value: 'oklch(0.546 0.135 162)' }, // #008857
    '700': { value: 'oklch(0.476 0.120 164)' }, // #00714a
    '800': { value: 'oklch(0.419 0.100 164)' }, // #005d3e
    '900': { value: 'oklch(0.350 0.078 166)' }, // #004731
    '950': { value: 'oklch(0.287 0.060 166)' }, // #003424
  },

  blue: {
    '50': { value: 'oklch(0.965 0.018 263)' }, // #edf4ff
    '100': { value: 'oklch(0.935 0.036 265)' }, // #deeaff
    '200': { value: 'oklch(0.881 0.068 267)' }, // #c4d7ff
    '300': { value: 'oklch(0.803 0.115 269)' }, // #a0bbff
    '400': { value: 'oklch(0.713 0.170 271)' }, // #7c99ff
    '500': { value: 'oklch(0.636 0.205 273)' }, // #667aff
    '600': { value: 'oklch(0.571 0.215 274)' }, // #5661f3
    '700': { value: 'oklch(0.501 0.205 275)' }, // #484cd5
    '800': { value: 'oklch(0.431 0.175 276)' }, // #3c3cad
    '900': { value: 'oklch(0.375 0.140 276)' }, // #313389
    '950': { value: 'oklch(0.258 0.095 277)' }, // #1a1b50
  },

  dark: {
    gray: {
      '50': { value: 'oklch(0.96 0.007 280)' }, // #f1f1f7
      '100': { value: 'oklch(0.90 0.007 280)' }, // #dddde3
      '200': { value: 'oklch(0.82 0.007 280)' }, // #c3c4c9
      '300': { value: 'oklch(0.72 0.007 280)' }, // #a3a4a9
      '400': { value: 'oklch(0.60 0.007 280)' }, // #7f8084
      '500': { value: 'oklch(0.48 0.007 280)' }, // #5d5d62
      '600': { value: 'oklch(0.38 0.007 280)' }, // #414246
      '700': { value: 'oklch(0.30 0.007 280)' }, // #2d2d31
      '800': { value: 'oklch(0.24 0.007 280)' }, // #1e1f23
      '900': { value: 'oklch(0.19 0.007 280)' }, // #131317
      '950': { value: 'oklch(0.15 0.007 280)' }, // #0a0b0e
    },

    brand: {
      '50': { value: 'oklch(0.82 0.065 282)' }, // #bdc0ee
      '100': { value: 'oklch(0.75 0.080 280)' }, // #a3a9e0
      '200': { value: 'oklch(0.67 0.095 278)' }, // #878fcf
      '300': { value: 'oklch(0.58 0.110 276)' }, // #6974bb
      '400': { value: 'oklch(0.50 0.115 274)' }, // #505ca4
      '500': { value: 'oklch(0.43 0.115 273)' }, // #3c488e
      '600': { value: 'oklch(0.37 0.100 272)' }, // #2e3a74
      '700': { value: 'oklch(0.31 0.085 270)' }, // #202c5b
      '800': { value: 'oklch(0.26 0.065 268)' }, // #172243
      '900': { value: 'oklch(0.21 0.050 266)' }, // #0d172f
      '950': { value: 'oklch(0.17 0.035 264)' }, // #080f1f
    },

    amber: {
      '50': { value: 'oklch(0.82 0.08 65)' }, // #e9ba8d
      '100': { value: 'oklch(0.75 0.10 65)' }, // #daa168
      '200': { value: 'oklch(0.67 0.13 64)' }, // #cc8231
      '300': { value: 'oklch(0.58 0.15 63)' }, // #b66100
      '400': { value: 'oklch(0.50 0.16 62)' }, // #a04600
      '500': { value: 'oklch(0.43 0.17 61)' }, // #8d2b00
      '600': { value: 'oklch(0.37 0.15 60)' }, // #741f00
      '700': { value: 'oklch(0.31 0.13 58)' }, // #5c1300
      '800': { value: 'oklch(0.26 0.10 56)' }, // #450f00
      '900': { value: 'oklch(0.21 0.07 54)' }, // #2f0b00
      '950': { value: 'oklch(0.17 0.05 52)' }, // #1f0700
    },

    red: {
      '50': { value: 'oklch(0.82 0.08 12)' }, // #f3afb6
      '100': { value: 'oklch(0.75 0.10 12)' }, // #e6939d
      '200': { value: 'oklch(0.67 0.13 12)' }, // #d87180
      '300': { value: 'oklch(0.58 0.16 12)' }, // #c64961
      '400': { value: 'oklch(0.50 0.17 12)' }, // #ae2749
      '500': { value: 'oklch(0.43 0.18 12)' }, // #990035
      '600': { value: 'oklch(0.37 0.17 12)' }, // #810027
      '700': { value: 'oklch(0.31 0.15 12)' }, // #67001b
      '800': { value: 'oklch(0.26 0.12 12)' }, // #4e0014
      '900': { value: 'oklch(0.21 0.08 12)' }, // #34020e
      '950': { value: 'oklch(0.17 0.06 12)' }, // #230208
    },

    green: {
      '50': { value: 'oklch(0.82 0.10 162)' }, // #86d9b0
      '100': { value: 'oklch(0.75 0.14 162)' }, // #45c992
      '200': { value: 'oklch(0.67 0.16 162)' }, // #00b276
      '300': { value: 'oklch(0.58 0.16 162)' }, // #00965c
      '400': { value: 'oklch(0.50 0.15 162)' }, // #007c47
      '500': { value: 'oklch(0.43 0.13 162)' }, // #006439
      '600': { value: 'oklch(0.37 0.11 164)' }, // #00512f
      '700': { value: 'oklch(0.31 0.09 164)' }, // #003e23
      '800': { value: 'oklch(0.26 0.07 166)' }, // #002e1c
      '900': { value: 'oklch(0.21 0.05 166)' }, // #001f13
      '950': { value: 'oklch(0.17 0.04 168)' }, // #00150c
    },

    blue: {
      '50': { value: 'oklch(0.82 0.08 272)' }, // #b2c2f9
      '100': { value: 'oklch(0.75 0.10 272)' }, // #98aaee
      '200': { value: 'oklch(0.67 0.13 272)' }, // #7a8fe5
      '300': { value: 'oklch(0.58 0.16 272)' }, // #5b70d8
      '400': { value: 'oklch(0.50 0.17 272)' }, // #4455c2
      '500': { value: 'oklch(0.43 0.18 272)' }, // #323db0
      '600': { value: 'oklch(0.37 0.17 272)' }, // #262d97
      '700': { value: 'oklch(0.31 0.15 272)' }, // #1b1f7a
      '800': { value: 'oklch(0.26 0.12 272)' }, // #14185c
      '900': { value: 'oklch(0.21 0.08 272)' }, // #0d123c
      '950': { value: 'oklch(0.17 0.06 272)' }, // #070b29
    },
  },
});

export const semanticColors = defineSemanticTokens.colors({
  'text.default': {
    value: {
      base: '{colors.gray.900}',
      _lightWhite: '{colors.gray.900}',
      _lightSnow: '#1c2638',
      _lightButter: '#33301a',
      _lightPeach: '#3c2018',
      _lightRose: '#371a2c',
      _lightLavender: '#201a41',
      _lightMint: '#1a3028',
      _lightLatte: '#2e2517',
      _darkBlack: '{colors.dark.gray.50}',
      _darkCharcoal: '#e4e4e7',
      _darkGraphite: '#e8e8eb',
      _darkMidnight: '#dce0f4',
      _darkNavy: '#d6dfec',
      _darkObsidian: '#e3def6',
      _darkStorm: '#d6dfec',
      _darkEspresso: '#f0e4d8',
    },
  },
  'text.subtle': {
    value: {
      base: '{colors.gray.700}',
      _lightWhite: '{colors.gray.700}',
      _lightSnow: '#3a4760',
      _lightButter: '#4a4530',
      _lightPeach: '#5a3830',
      _lightRose: '#54324c',
      _lightLavender: '#373262',
      _lightMint: '#304840',
      _lightLatte: '#4a3c28',
      _darkBlack: '{colors.dark.gray.100}',
      _darkCharcoal: '#c8c8cc',
      _darkGraphite: '#d0d0d4',
      _darkMidnight: '#c0c4e0',
      _darkNavy: '#bbc7dc',
      _darkObsidian: '#cdc6e3',
      _darkStorm: '#bac7dc',
      _darkEspresso: '#dcccc0',
    },
  },
  'text.muted': {
    value: {
      base: '{colors.gray.600}',
      _lightWhite: '{colors.gray.600}',
      _lightSnow: '#526178',
      _lightButter: '#5e5844',
      _lightPeach: '#6c4840',
      _lightRose: '#67485b',
      _lightLavender: '#47446a',
      _lightMint: '#445c54',
      _lightLatte: '#5d5342',
      _darkBlack: '{colors.dark.gray.200}',
      _darkCharcoal: '#a8a8ac',
      _darkGraphite: '#b0b0b4',
      _darkMidnight: '#9798c0',
      _darkNavy: '#95a6c0',
      _darkObsidian: '#ac9cc7',
      _darkStorm: '#94a7c0',
      _darkEspresso: '#c0b0a0',
    },
  },
  'text.faint': {
    value: {
      base: '{colors.gray.500}',
      _lightWhite: '{colors.gray.500}',
      _lightSnow: '#707d94',
      _lightButter: '#787058',
      _lightPeach: '#886058',
      _lightRose: '#785f6a',
      _lightLavender: '#5c5979',
      _lightMint: '#5c7468',
      _lightLatte: '#726656',
      _darkBlack: '{colors.dark.gray.300}',
      _darkCharcoal: '#888890',
      _darkGraphite: '#909098',
      _darkMidnight: '#7778a0',
      _darkNavy: '#7586a0',
      _darkObsidian: '#8c7ca7',
      _darkStorm: '#7487a0',
      _darkEspresso: '#a09080',
    },
  },
  'text.disabled': {
    value: {
      base: '{colors.gray.400}',
      _lightWhite: '{colors.gray.400}',
      _lightSnow: '#8d97aa',
      _lightButter: '#968e78',
      _lightPeach: '#a07870',
      _lightRose: '#917688',
      _lightLavender: '#787591',
      _lightMint: '#7c9088',
      _lightLatte: '#8a7d6d',
      _darkBlack: '{colors.dark.gray.400}',
      _darkCharcoal: '#606068',
      _darkGraphite: '#686870',
      _darkMidnight: '#595a83',
      _darkNavy: '#4d5e78',
      _darkObsidian: '#65547f',
      _darkStorm: '#4c5f78',
      _darkEspresso: '#786858',
    },
  },
  'text.bright': {
    value: {
      base: '{colors.white}',
      _lightWhite: '{colors.white}',
      _lightSnow: '#ffffff',
      _lightButter: '#ffffff',
      _lightPeach: '#ffffff',
      _lightRose: '#ffffff',
      _lightLavender: '#fdfeff',
      _lightMint: '#ffffff',
      _lightLatte: '#fefefe',
      _darkBlack: '{colors.dark.gray.50}',
      _darkCharcoal: '#e8e8eb',
      _darkGraphite: '#ececedee',
      _darkMidnight: '#e0e4f8',
      _darkNavy: '#dae3f0',
      _darkObsidian: '#e7e2fa',
      _darkStorm: '#dae3f0',
      _darkEspresso: '#f4e8dc',
    },
  },
  'text.danger': { value: { base: '{colors.red.500}', _dark: '{colors.dark.red.300}' } },
  'text.success': { value: { base: '{colors.green.700}', _dark: '{colors.dark.green.300}' } },
  'text.link': { value: { base: '{colors.blue.600}', _dark: '{colors.dark.blue.400}' } },
  'text.brand': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.300}' } },

  'surface.default': {
    value: {
      base: '{colors.white}',
      _lightWhite: '{colors.white}',
      _lightSnow: '#f8f9fc',
      _lightButter: '#fffef8',
      _lightPeach: '#fff8f4',
      _lightRose: '#fdf8fb',
      _lightLavender: '#f7f7fc',
      _lightMint: '#f9fdfa',
      _lightLatte: '#fbf9f4',
      _darkBlack: '{colors.dark.gray.900}',
      _darkCharcoal: '#1a1a1c',
      _darkGraphite: '#222226',
      _darkMidnight: '#14141e',
      _darkNavy: '#0e1420',
      _darkObsidian: '#181621',
      _darkStorm: '#171c22',
      _darkEspresso: '#1c1610',
    },
  },
  'surface.subtle': {
    value: {
      base: '{colors.gray.50}',
      _lightWhite: '{colors.gray.50}',
      _lightSnow: '#f2f4f7',
      _lightButter: '#fbf9ef',
      _lightPeach: '#fbf2ed',
      _lightRose: '#f7f2f6',
      _lightLavender: '#f1f1f8',
      _lightMint: '#f3f9f5',
      _lightLatte: '#f5f3ec',
      _darkBlack: '{colors.dark.gray.800}',
      _darkCharcoal: '#202022',
      _darkGraphite: '#28282c',
      _darkMidnight: '#1a1a26',
      _darkNavy: '#121828',
      _darkObsidian: '#1d1929',
      _darkStorm: '#1d222a',
      _darkEspresso: '#201c18',
    },
  },
  'surface.muted': {
    value: {
      base: '{colors.gray.100}',
      _lightWhite: '{colors.gray.100}',
      _lightSnow: '#eaecf2',
      _lightButter: '#f7f5e7',
      _lightPeach: '#f8eae4',
      _lightRose: '#f3ebf1',
      _lightLavender: '#e9e9f4',
      _lightMint: '#ecf6f0',
      _lightLatte: '#eeece4',
      _darkBlack: '{colors.dark.gray.700}',
      _darkCharcoal: '#262628',
      _darkGraphite: '#2e2e32',
      _darkMidnight: '#20202e',
      _darkNavy: '#161c2c',
      _darkObsidian: '#231f31',
      _darkStorm: '#232a32',
      _darkEspresso: '#26221c',
    },
  },
  'surface.dark': {
    value: {
      base: '{colors.gray.700}',
      _lightWhite: '{colors.gray.700}',
      _lightSnow: '#343f58',
      _lightButter: '#443c28',
      _lightPeach: '#502c20',
      _lightRose: '#3d293a',
      _lightLavender: '#2c2949',
      _lightMint: '#284038',
      _lightLatte: '#373129',
      _darkBlack: '{colors.dark.gray.700}',
      _darkCharcoal: '#38383c',
      _darkGraphite: '#3c3c42',
      _darkMidnight: '#2b2c4a',
      _darkNavy: '#202b40',
      _darkObsidian: '#362e4d',
      _darkStorm: '#2e3744',
      _darkEspresso: '#3c3028',
    },
  },

  'interactive.hover': {
    value: {
      base: '{colors.gray.200}',
      _lightWhite: '{colors.gray.200}',
      _lightSnow: '#dde2ea',
      _lightButter: '#ece8d0',
      _lightPeach: '#f0dcd0',
      _lightRose: '#e6d9e4',
      _lightLavender: '#dadbf1',
      _lightMint: '#d4e8de',
      _lightLatte: '#e0dcd0',
      _darkBlack: '{colors.dark.gray.600}',
      _darkCharcoal: '#3a3a3e',
      _darkGraphite: '#3e3e44',
      _darkMidnight: '#2d2e48',
      _darkNavy: '#202b40',
      _darkObsidian: '#322a4b',
      _darkStorm: '#2a3340',
      _darkEspresso: '#382c24',
    },
  },
  'interactive.disabled': {
    value: {
      base: '{colors.gray.200}',
      _lightWhite: '{colors.gray.200}',
      _lightSnow: '#dde2ea',
      _lightButter: '#ece8d0',
      _lightPeach: '#f0dcd0',
      _lightRose: '#e6d9e4',
      _lightLavender: '#dadbf1',
      _lightMint: '#d4e8de',
      _lightLatte: '#e0dcd0',
      _darkBlack: '{colors.dark.gray.800}',
      _darkCharcoal: '#222224',
      _darkGraphite: '#2a2a2e',
      _darkMidnight: '#181830',
      _darkNavy: '#0f1728',
      _darkObsidian: '#201a32',
      _darkStorm: '#151c28',
      _darkEspresso: '#241c14',
    },
  },

  'accent.brand.default': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.400}' } },
  'accent.brand.hover': { value: { base: '{colors.brand.600}', _dark: '{colors.dark.brand.500}' } },
  'accent.brand.active': { value: { base: '{colors.brand.700}', _dark: '{colors.dark.brand.600}' } },
  'accent.brand.subtle': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.brand.900}' } },
  'accent.info.default': { value: { base: '{colors.blue.500}', _dark: '{colors.dark.blue.200}' } },
  'accent.info.subtle': { value: { base: '{colors.blue.50}', _dark: '{colors.dark.blue.900}' } },
  'accent.danger.default': { value: { base: '{colors.red.600}', _dark: '{colors.dark.red.400}' } },
  'accent.danger.hover': { value: { base: '{colors.red.500}', _dark: '{colors.dark.red.500}' } },
  'accent.danger.active': { value: { base: '{colors.red.700}', _dark: '{colors.dark.red.600}' } },
  'accent.danger.subtle': { value: { base: '{colors.red.50}', _dark: '{colors.dark.red.900}' } },
  'accent.success.default': { value: { base: '{colors.green.700}', _dark: '{colors.dark.green.300}' } },
  'accent.success.subtle': { value: { base: '{colors.green.50}', _dark: '{colors.dark.green.900}' } },

  'border.default': {
    value: {
      base: '{colors.gray.200}',
      _lightWhite: '{colors.gray.200}',
      _lightSnow: '#d8dce6',
      _lightButter: '#e2dcc8',
      _lightPeach: '#e8ccc0',
      _lightRose: '#dacdd8',
      _lightLavender: '#cbcce0',
      _lightMint: '#c8dcd2',
      _lightLatte: '#d4d0c6',
      _darkBlack: '{colors.dark.gray.700}',
      _darkCharcoal: '#323236',
      _darkGraphite: '#383840',
      _darkMidnight: '#272840',
      _darkNavy: '#202b3d',
      _darkObsidian: '#302a42',
      _darkStorm: '#28313e',
      _darkEspresso: '#342820',
    },
  },
  'border.strong': {
    value: {
      base: '{colors.gray.300}',
      _lightWhite: '{colors.gray.300}',
      _lightSnow: '#b9c0d0',
      _lightButter: '#ccc4a8',
      _lightPeach: '#d4a898',
      _lightRose: '#c1a9bb',
      _lightLavender: '#adadc9',
      _lightMint: '#a4c8b8',
      _lightLatte: '#bcb8a8',
      _darkBlack: '{colors.dark.gray.600}',
      _darkCharcoal: '#424248',
      _darkGraphite: '#48484e',
      _darkMidnight: '#373858',
      _darkNavy: '#293748',
      _darkObsidian: '#3f3a5a',
      _darkStorm: '#2f3b4c',
      _darkEspresso: '#443830',
    },
  },
  'border.subtle': {
    value: {
      base: '{colors.gray.100}',
      _lightWhite: '{colors.gray.100}',
      _lightSnow: '#e4e8f0',
      _lightButter: '#ece8d4',
      _lightPeach: '#f0dcd2',
      _lightRose: '#e4d9e3',
      _lightLavender: '#d7d8ec',
      _lightMint: '#d8ece2',
      _lightLatte: '#e0dcd2',
      _darkBlack: '{colors.dark.gray.800}',
      _darkCharcoal: '#242428',
      _darkGraphite: '#2c2c30',
      _darkMidnight: '#1c1c30',
      _darkNavy: '#17202d',
      _darkObsidian: '#241e32',
      _darkStorm: '#1f262e',
      _darkEspresso: '#281c14',
    },
  },
  'border.brand': { value: { base: '{colors.brand.600}', _dark: '{colors.dark.brand.400}' } },
  'border.danger': { value: { base: '{colors.red.600}', _dark: '{colors.dark.red.400}' } },

  'shadow.default': { value: { base: '{colors.gray.950}', _dark: '{colors.dark.gray.950}' } },

  'decoration.grid.brand': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.gray.700}' } },
  'decoration.grid.brand.subtle': { value: { base: '{colors.brand.50}', _dark: '{colors.dark.gray.800}' } },
});
