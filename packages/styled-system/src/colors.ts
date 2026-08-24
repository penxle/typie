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

  emerald: {
    '50': { value: 'oklch(0.965 0.025 152)' }, // #e8f9eb
    '100': { value: 'oklch(0.905 0.072 152)' }, // #bdeec8
    '200': { value: 'oklch(0.855 0.116 152)' }, // #93e6aa
    '300': { value: 'oklch(0.795 0.174 152)' }, // #50db82
    '400': { value: 'oklch(0.700 0.166 152)' }, // #32bb68
    '500': { value: 'oklch(0.620 0.147 152)' }, // #299e58
    '600': { value: 'oklch(0.560 0.133 152)' }, // #228a4b
    '700': { value: 'oklch(0.490 0.116 152)' }, // #1b723e
    '800': { value: 'oklch(0.420 0.100 152)' }, // #135c30
    '900': { value: 'oklch(0.360 0.085 152)' }, // #0e4926
    '950': { value: 'oklch(0.250 0.060 152)' }, // #042912
  },

  pink: {
    '50': { value: 'oklch(0.965 0.025 330)' }, // #feeefc
    '100': { value: 'oklch(0.905 0.072 330)' }, // #fccff7
    '200': { value: 'oklch(0.855 0.116 330)' }, // #fbb4f4
    '300': { value: 'oklch(0.795 0.174 330)' }, // #f98ff1
    '400': { value: 'oklch(0.730 0.246 330)' }, // #f85def
    '500': { value: 'oklch(0.665 0.273 330)' }, // #e832e0
    '600': { value: 'oklch(0.600 0.246 330)' }, // #ca2bc4
    '700': { value: 'oklch(0.530 0.218 330)' }, // #ab22a6
    '800': { value: 'oklch(0.460 0.189 330)' }, // #8d1b88
    '900': { value: 'oklch(0.395 0.162 330)' }, // #72146e
    '950': { value: 'oklch(0.265 0.109 330)' }, // #40073e
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

  // 다크 스케일 공통 설계: 명도 램프는 전 패밀리 동일(0.83→0.19), 채도는 라이트보다 높게(어두운 배경에선
  // 지각 채도가 떨어짐 — 헌트 효과), 색상은 저명도 표류 없이 고정. 전 단계 sRGB 색역 수치 검증됨(초과분은 최대 채도로 클램프).
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

    // 다크 스케일은 라이트보다 채도를 더 싣는다 — 어두운 배경에선 지각 채도가 떨어져(헌트 효과) 같은 채도가 탁하게 읽힌다.
    // 색상도 브랜드 보라(272~282)에 고정한다: 저명도에서 청색으로 흘리면 잉크빛이 된다. 전 단계 sRGB 색역 수치 검증됨.
    brand: {
      '50': { value: 'oklch(0.83 0.080 282)' }, // #bec2fb
      '100': { value: 'oklch(0.76 0.110 281)' }, // #a5a9f6
      '200': { value: 'oklch(0.68 0.130 280)' }, // #898ee7
      '300': { value: 'oklch(0.60 0.145 279)' }, // #6f74d4
      '400': { value: 'oklch(0.55 0.155 278)' }, // #5f64c9
      '500': { value: 'oklch(0.48 0.155 277)' }, // #4b4fb2
      '600': { value: 'oklch(0.41 0.145 276)' }, // #383c97
      '700': { value: 'oklch(0.35 0.125 275)' }, // #2a2f7a
      '800': { value: 'oklch(0.29 0.105 274)' }, // #1d235e
      '900': { value: 'oklch(0.24 0.085 273)' }, // #131947
      '950': { value: 'oklch(0.19 0.065 272)' }, // #0a1030
    },

    amber: {
      '50': { value: 'oklch(0.83 0.090 66)' }, // #f0bc88
      '100': { value: 'oklch(0.76 0.110 65)' }, // #e1a263
      '200': { value: 'oklch(0.68 0.140 64)' }, // #d38327
      '300': { value: 'oklch(0.60 0.136 63)' }, // #b86b03
      '400': { value: 'oklch(0.55 0.126 62)' }, // #a45e02
      '500': { value: 'oklch(0.48 0.111 61)' }, // #894c01
      '600': { value: 'oklch(0.41 0.096 60)' }, // #6f3b00
      '700': { value: 'oklch(0.35 0.083 59)' }, // #592e00
      '800': { value: 'oklch(0.29 0.069 58)' }, // #442100
      '900': { value: 'oklch(0.24 0.058 57)' }, // #331700
      '950': { value: 'oklch(0.19 0.046 56)' }, // #230d00
    },

    red: {
      '50': { value: 'oklch(0.83 0.090 12)' }, // #fcafb8
      '100': { value: 'oklch(0.76 0.120 12)' }, // #f3909d
      '200': { value: 'oklch(0.68 0.150 12)' }, // #e56c7f
      '300': { value: 'oklch(0.60 0.180 12)' }, // #d54563
      '400': { value: 'oklch(0.55 0.190 12)' }, // #c72c53
      '500': { value: 'oklch(0.48 0.190 12)' }, // #ae0440
      '600': { value: 'oklch(0.41 0.163 12)' }, // #8d0132
      '700': { value: 'oklch(0.35 0.139 12)' }, // #710127
      '800': { value: 'oklch(0.29 0.116 12)' }, // #56001b
      '900': { value: 'oklch(0.24 0.096 12)' }, // #410013
      '950': { value: 'oklch(0.19 0.076 12)' }, // #2d000a
    },

    green: {
      '50': { value: 'oklch(0.83 0.110 162)' }, // #81deb1
      '100': { value: 'oklch(0.76 0.150 162)' }, // #3ace93
      '200': { value: 'oklch(0.68 0.149 162)' }, // #01b47b
      '300': { value: 'oklch(0.60 0.131 162)' }, // #039868
      '400': { value: 'oklch(0.55 0.120 162)' }, // #03875b
      '500': { value: 'oklch(0.48 0.105 162)' }, // #016f4b
      '600': { value: 'oklch(0.41 0.089 162)' }, // #02593b
      '700': { value: 'oklch(0.35 0.076 162)' }, // #01462e
      '800': { value: 'oklch(0.29 0.063 162)' }, // #013521
      '900': { value: 'oklch(0.24 0.052 162)' }, // #012717
      '950': { value: 'oklch(0.19 0.041 162)' }, // #00190e
    },

    emerald: {
      '50': { value: 'oklch(0.83 0.090 152)' }, // #9bd9ab
      '100': { value: 'oklch(0.76 0.130 152)' }, // #6bc987
      '200': { value: 'oklch(0.68 0.160 152)' }, // #32b364
      '300': { value: 'oklch(0.60 0.157 152)' }, // #049a4e
      '400': { value: 'oklch(0.55 0.144 152)' }, // #028844
      '500': { value: 'oklch(0.48 0.126 152)' }, // #017137
      '600': { value: 'oklch(0.41 0.107 152)' }, // #025a2b
      '700': { value: 'oklch(0.35 0.092 152)' }, // #004721
      '800': { value: 'oklch(0.29 0.076 152)' }, // #003617
      '900': { value: 'oklch(0.24 0.063 152)' }, // #00270f
      '950': { value: 'oklch(0.19 0.050 152)' }, // #001a08
    },

    pink: {
      '50': { value: 'oklch(0.83 0.080 330)' }, // #e6b5e0
      '100': { value: 'oklch(0.76 0.120 330)' }, // #dc95d5
      '200': { value: 'oklch(0.68 0.190 330)' }, // #d665cf
      '300': { value: 'oklch(0.60 0.210 330)' }, // #c142ba
      '400': { value: 'oklch(0.55 0.210 330)' }, // #b030aa
      '500': { value: 'oklch(0.48 0.190 330)' }, // #94228f
      '600': { value: 'oklch(0.41 0.170 330)' }, // #791475
      '700': { value: 'oklch(0.35 0.150 330)' }, // #62095e
      '800': { value: 'oklch(0.29 0.120 330)' }, // #490847
      '900': { value: 'oklch(0.24 0.100 330)' }, // #370435
      '950': { value: 'oklch(0.19 0.080 330)' }, // #260224
    },

    blue: {
      '50': { value: 'oklch(0.83 0.084 272)' }, // #b4c5ff
      '100': { value: 'oklch(0.76 0.110 272)' }, // #99adf7
      '200': { value: 'oklch(0.68 0.140 272)' }, // #7b91ee
      '300': { value: 'oklch(0.60 0.170 272)' }, // #5f74e4
      '400': { value: 'oklch(0.55 0.180 272)' }, // #5063d9
      '500': { value: 'oklch(0.48 0.180 272)' }, // #3e4dc1
      '600': { value: 'oklch(0.41 0.170 272)' }, // #2f39a4
      '700': { value: 'oklch(0.35 0.150 272)' }, // #232b87
      '800': { value: 'oklch(0.29 0.130 272)' }, // #191e6a
      '900': { value: 'oklch(0.24 0.100 272)' }, // #11164d
      '950': { value: 'oklch(0.19 0.080 272)' }, // #090d37
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
  // 다크의 유색 글자는 밝고 선명한 100 — 본문 속에 앉는 링크만 한 단 깊은 200
  'text.danger': { value: { base: '{colors.red.500}', _dark: '{colors.dark.red.100}' } },
  'text.success': { value: { base: '{colors.green.700}', _dark: '{colors.dark.green.100}' } },
  'text.link': { value: { base: '{colors.blue.600}', _dark: '{colors.dark.blue.200}' } },
  'text.brand': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.100}' } },
  'text.emerald': { value: { base: '{colors.emerald.700}', _dark: '{colors.dark.emerald.100}' } },
  'text.pink': { value: { base: '{colors.pink.700}', _dark: '{colors.dark.pink.100}' } },

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
  'surface.inverse': {
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

  // 다크의 인터랙션 램프는 라이트의 반전이다 — 어두운 바탕에서 강조는 밝아지는 방향이라, hover·active가 default보다 밝은 단계로 간다
  'accent.brand.default': { value: { base: '{colors.brand.500}', _dark: '{colors.dark.brand.400}' } },
  'accent.brand.hover': { value: { base: '{colors.brand.600}', _dark: '{colors.dark.brand.300}' } },
  'accent.brand.active': { value: { base: '{colors.brand.700}', _dark: '{colors.dark.brand.200}' } },
  // 다크의 subtle 면은 불투명 단계가 아니라 선명한 밝은 톤의 알파다 — 불투명 저명도는 탁하고, 불투명 고명도는 표면에서 눈부시다.
  // 알파는 광량을 표면에 맡기고 색기만 얹는다
  'accent.brand.subtle': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.brand.300/30}' } },
  // 전 패밀리 공통 3규칙(브랜드와 동일): 채움 램프는 다크에서 반전(400→300→200), subtle 면은 선명한 300톤의 30% 알파
  'accent.emerald.default': { value: { base: '{colors.emerald.500}', _dark: '{colors.dark.emerald.400}' } },
  'accent.emerald.subtle': { value: { base: '{colors.emerald.100}', _dark: '{colors.dark.emerald.300/30}' } },
  'accent.pink.default': { value: { base: '{colors.pink.500}', _dark: '{colors.dark.pink.400}' } },
  'accent.pink.hover': { value: { base: '{colors.pink.600}', _dark: '{colors.dark.pink.300}' } },
  'accent.pink.active': { value: { base: '{colors.pink.700}', _dark: '{colors.dark.pink.200}' } },
  'accent.pink.subtle': { value: { base: '{colors.pink.100}', _dark: '{colors.dark.pink.300/30}' } },
  'accent.info.default': { value: { base: '{colors.blue.500}', _dark: '{colors.dark.blue.400}' } },
  'accent.info.subtle': { value: { base: '{colors.blue.50}', _dark: '{colors.dark.blue.300/30}' } },
  'accent.danger.default': { value: { base: '{colors.red.600}', _dark: '{colors.dark.red.400}' } },
  'accent.danger.hover': { value: { base: '{colors.red.700}', _dark: '{colors.dark.red.300}' } },
  'accent.danger.active': { value: { base: '{colors.red.800}', _dark: '{colors.dark.red.200}' } },
  'accent.danger.subtle': { value: { base: '{colors.red.50}', _dark: '{colors.dark.red.300/30}' } },
  'accent.warning.default': { value: { base: '{colors.amber.600}', _dark: '{colors.dark.amber.400}' } },
  'accent.warning.subtle': { value: { base: '{colors.amber.50}', _dark: '{colors.dark.amber.300/30}' } },
  'accent.success.default': { value: { base: '{colors.green.700}', _dark: '{colors.dark.green.400}' } },
  'accent.success.subtle': { value: { base: '{colors.green.50}', _dark: '{colors.dark.green.300/30}' } },

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
      _darkBlack: '{colors.dark.gray.600}',
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
      _darkBlack: '{colors.dark.gray.500}',
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
  'border.emerald': { value: { base: '{colors.emerald.600}', _dark: '{colors.dark.emerald.400}' } },
  'border.pink': { value: { base: '{colors.pink.600}', _dark: '{colors.dark.pink.400}' } },

  'shadow.default': { value: { base: '{colors.gray.950}', _dark: '{colors.dark.gray.950}' } },

  'decoration.grid.brand': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.gray.700}' } },
  'decoration.grid.brand.subtle': { value: { base: '{colors.brand.50}', _dark: '{colors.dark.gray.800}' } },

  'palette.gray': { value: { base: '#71717a', _dark: '#b4b4bc' } },
  'palette.red': { value: { base: '#ef4444', _dark: '#fca5a5' } },
  'palette.orange': { value: { base: '#f97316', _dark: '#fdba74' } },
  'palette.yellow': { value: { base: '#eab308', _dark: '#fde047' } },
  'palette.green': { value: { base: '#22c55e', _dark: '#86efac' } },
  'palette.blue': { value: { base: '#3b82f6', _dark: '#93c5fd' } },
  'palette.purple': { value: { base: '#8b5cf6', _dark: '#c4b5fd' } },
});
