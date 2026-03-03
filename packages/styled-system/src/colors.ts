import { defineSemanticTokens, defineTokens } from '@pandacss/dev';

export const colors = defineTokens.colors({
  current: { value: 'currentColor' },

  white: { value: '#fff' },
  black: { value: '#000' },
  transparent: { value: 'rgb(0 0 0 / 0)' },

  brand: {
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
      '50': { value: 'oklch(0.96 0.006 266)' }, // #f0f0f2
      '100': { value: 'oklch(0.90 0.006 266)' }, // #dadade
      '200': { value: 'oklch(0.82 0.006 266)' }, // #bfc0c4
      '300': { value: 'oklch(0.72 0.006 266)' }, // #9fa0a6
      '400': { value: 'oklch(0.60 0.006 266)' }, // #7e8086
      '500': { value: 'oklch(0.48 0.006 266)' }, // #5e6066
      '600': { value: 'oklch(0.38 0.006 266)' }, // #464850
      '700': { value: 'oklch(0.30 0.006 266)' }, // #383a42
      '800': { value: 'oklch(0.24 0.006 266)' }, // #2a2c32
      '900': { value: 'oklch(0.19 0.006 266)' }, // #1e2026
      '950': { value: 'oklch(0.15 0.006 266)' }, // #16181c
    },

    brand: {
      '50': { value: 'oklch(0.85 0.08 70)' }, // #ffd199
      '100': { value: 'oklch(0.78 0.10 70)' }, // #f4b877
      '200': { value: 'oklch(0.70 0.13 70)' }, // #e39d55
      '300': { value: 'oklch(0.62 0.16 70)' }, // #d18239
      '400': { value: 'oklch(0.55 0.18 70)' }, // #bd6922
      '500': { value: 'oklch(0.48 0.19 70)' }, // #a5540f
      '600': { value: 'oklch(0.42 0.18 60)' }, // #8a4308
      '700': { value: 'oklch(0.36 0.16 55)' }, // #6d3404
      '800': { value: 'oklch(0.30 0.12 50)' }, // #512701
      '900': { value: 'oklch(0.24 0.08 48)' }, // #361b00
      '950': { value: 'oklch(0.19 0.06 46)' }, // #221100
    },

    amber: {
      '50': { value: 'oklch(0.85 0.08 70)' }, // #ffd199
      '100': { value: 'oklch(0.78 0.10 70)' }, // #f4b877
      '200': { value: 'oklch(0.70 0.13 70)' }, // #e39d55
      '300': { value: 'oklch(0.62 0.16 70)' }, // #d18239
      '400': { value: 'oklch(0.55 0.18 70)' }, // #bd6922
      '500': { value: 'oklch(0.48 0.19 70)' }, // #a5540f
      '600': { value: 'oklch(0.42 0.18 60)' }, // #8a4308
      '700': { value: 'oklch(0.36 0.16 55)' }, // #6d3404
      '800': { value: 'oklch(0.30 0.12 50)' }, // #512701
      '900': { value: 'oklch(0.24 0.08 48)' }, // #361b00
      '950': { value: 'oklch(0.19 0.06 46)' }, // #221100
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
  selection: { value: { base: 'rgba(153, 204, 255, 0.3)' } },

  'text.default': {
    value: {
      base: '{colors.gray.900}',
      _lightWhite: '{colors.gray.900}',
      _lightSnow: '#1a2638',
      _lightButter: '#33301a',
      _lightPeach: '#3c2018',
      _lightRose: '#3a1828',
      _lightLavender: '#241840',
      _lightMint: '#1a3028',
      _lightLatte: '#302418',
      _darkBlack: '{colors.dark.gray.50}',
      _darkCharcoal: '#e4e4e6',
      _darkGraphite: '#e8e8ea',
      _darkMidnight: '#dce0f4',
      _darkNavy: '#d4e0ec',
      _darkObsidian: '#e8dcf4',
      _darkStorm: '#d4e0ec',
      _darkEspresso: '#f0e4d8',
    },
  },
  'text.subtle': {
    value: {
      base: '{colors.gray.700}',
      _lightWhite: '{colors.gray.700}',
      _lightSnow: '#384860',
      _lightButter: '#4a4530',
      _lightPeach: '#5a3830',
      _lightRose: '#583048',
      _lightLavender: '#3c3060',
      _lightMint: '#304840',
      _lightLatte: '#4a3c28',
      _darkBlack: '{colors.dark.gray.100}',
      _darkCharcoal: '#c8c8cc',
      _darkGraphite: '#d0d0d4',
      _darkMidnight: '#c0c4e0',
      _darkNavy: '#b8c8dc',
      _darkObsidian: '#d4c4e0',
      _darkStorm: '#b8c8dc',
      _darkEspresso: '#dcccc0',
    },
  },
  'text.muted': {
    value: {
      base: '{colors.gray.600}',
      _lightWhite: '{colors.gray.600}',
      _lightSnow: '#506278',
      _lightButter: '#5e5844',
      _lightPeach: '#6c4840',
      _lightRose: '#5c3848',
      _lightLavender: '#4c4268',
      _lightMint: '#445c54',
      _lightLatte: '#524838',
      _darkBlack: '{colors.dark.gray.200}',
      _darkCharcoal: '#a8a8ac',
      _darkGraphite: '#b0b0b4',
      _darkMidnight: '#9898c0',
      _darkNavy: '#90a8c0',
      _darkObsidian: '#b898c0',
      _darkStorm: '#90a8c0',
      _darkEspresso: '#c0b0a0',
    },
  },
  'text.faint': {
    value: {
      base: '{colors.gray.500}',
      _lightWhite: '{colors.gray.500}',
      _lightSnow: '#6e7e94',
      _lightButter: '#787058',
      _lightPeach: '#886058',
      _lightRose: '#6c5058',
      _lightLavender: '#605878',
      _lightMint: '#5c7468',
      _lightLatte: '#685c4c',
      _darkBlack: '{colors.dark.gray.300}',
      _darkCharcoal: '#888890',
      _darkGraphite: '#909098',
      _darkMidnight: '#7878a0',
      _darkNavy: '#7088a0',
      _darkObsidian: '#9878a0',
      _darkStorm: '#7088a0',
      _darkEspresso: '#a09080',
    },
  },
  'text.disabled': {
    value: {
      base: '{colors.gray.400}',
      _lightWhite: '{colors.gray.400}',
      _lightSnow: '#8c98aa',
      _lightButter: '#968e78',
      _lightPeach: '#a07870',
      _lightRose: '#886878',
      _lightLavender: '#7c7490',
      _lightMint: '#7c9088',
      _lightLatte: '#847868',
      _darkBlack: '{colors.dark.gray.400}',
      _darkCharcoal: '#606068',
      _darkGraphite: '#686870',
      _darkMidnight: '#505078',
      _darkNavy: '#486078',
      _darkObsidian: '#705078',
      _darkStorm: '#486078',
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
      _lightLavender: '#fefefe',
      _lightMint: '#ffffff',
      _lightLatte: '#fefefe',
      _darkBlack: '{colors.dark.gray.50}',
      _darkCharcoal: '#e8e8ea',
      _darkGraphite: '#ecececee',
      _darkMidnight: '#e0e4f8',
      _darkNavy: '#d8e4f0',
      _darkObsidian: '#ece0f8',
      _darkStorm: '#d8e4f0',
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
      _lightSnow: '#f8f9fb',
      _lightButter: '#fffef8',
      _lightPeach: '#fff8f4',
      _lightRose: '#fef8fa',
      _lightLavender: '#f8f7fc',
      _lightMint: '#f9fdfa',
      _lightLatte: '#fbf9f4',
      _darkBlack: '#121212',
      _darkCharcoal: '#1a1a1c',
      _darkGraphite: '#222226',
      _darkMidnight: '#14141e',
      _darkNavy: '#0c1420',
      _darkObsidian: '#1a1520',
      _darkStorm: '#161c22',
      _darkEspresso: '#1c1610',
    },
  },
  'surface.subtle': {
    value: {
      base: '{colors.gray.50}',
      _lightWhite: '{colors.gray.50}',
      _lightSnow: '#f2f4f7',
      _lightButter: '#fefcf2',
      _lightPeach: '#faf2ec',
      _lightRose: '#f8f2f5',
      _lightLavender: '#f2f1f8',
      _lightMint: '#f4faf6',
      _lightLatte: '#f5f3ec',
      _darkBlack: '#1a1a1a',
      _darkCharcoal: '#202022',
      _darkGraphite: '#28282c',
      _darkMidnight: '#1a1a26',
      _darkNavy: '#101828',
      _darkObsidian: '#201828',
      _darkStorm: '#1c222a',
      _darkEspresso: '#201c18',
    },
  },
  'surface.muted': {
    value: {
      base: '{colors.gray.100}',
      _lightWhite: '{colors.gray.100}',
      _lightSnow: '#eaecf2',
      _lightButter: '#faf8ea',
      _lightPeach: '#f6ebe2',
      _lightRose: '#f4ebf0',
      _lightLavender: '#eae9f4',
      _lightMint: '#ecf6f0',
      _lightLatte: '#eeece4',
      _darkBlack: '#222222',
      _darkCharcoal: '#262628',
      _darkGraphite: '#2e2e32',
      _darkMidnight: '#20202e',
      _darkNavy: '#141c2c',
      _darkObsidian: '#261e30',
      _darkStorm: '#222a32',
      _darkEspresso: '#26221c',
    },
  },
  'surface.dark': {
    value: {
      base: '{colors.gray.700}',
      _lightWhite: '{colors.gray.700}',
      _lightSnow: '#324058',
      _lightButter: '#443c28',
      _lightPeach: '#502c20',
      _lightRose: '#402838',
      _lightLavender: '#302848',
      _lightMint: '#284038',
      _lightLatte: '#38302a',
      _darkBlack: '{colors.dark.gray.700}',
      _darkCharcoal: '#38383c',
      _darkGraphite: '#3c3c42',
      _darkMidnight: '#2c2c4a',
      _darkNavy: '#1c2c40',
      _darkObsidian: '#3c2c4a',
      _darkStorm: '#2c3844',
      _darkEspresso: '#3c3028',
    },
  },

  'interactive.hover': {
    value: {
      base: '{colors.gray.200}',
      _lightWhite: '{colors.gray.200}',
      _lightSnow: '#dce2ea',
      _lightButter: '#ece8d0',
      _lightPeach: '#f0dcd0',
      _lightRose: '#e8d8e2',
      _lightLavender: '#dcdaf0',
      _lightMint: '#d4e8de',
      _lightLatte: '#e0dcd0',
      _darkBlack: '{colors.dark.gray.600}',
      _darkCharcoal: '#3a3a3e',
      _darkGraphite: '#3e3e44',
      _darkMidnight: '#2e2e48',
      _darkNavy: '#1c2c40',
      _darkObsidian: '#382848',
      _darkStorm: '#283440',
      _darkEspresso: '#382c24',
    },
  },
  'interactive.disabled': {
    value: {
      base: '{colors.gray.200}',
      _lightWhite: '{colors.gray.200}',
      _lightSnow: '#dce2ea',
      _lightButter: '#ece8d0',
      _lightPeach: '#f0dcd0',
      _lightRose: '#e8d8e2',
      _lightLavender: '#dcdaf0',
      _lightMint: '#d4e8de',
      _lightLatte: '#e0dcd0',
      _darkBlack: '{colors.dark.gray.800}',
      _darkCharcoal: '#222224',
      _darkGraphite: '#2a2a2e',
      _darkMidnight: '#181830',
      _darkNavy: '#0c1828',
      _darkObsidian: '#241830',
      _darkStorm: '#141c28',
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
      _lightRose: '#dcccd6',
      _lightLavender: '#cccce0',
      _lightMint: '#c8dcd2',
      _lightLatte: '#d4d0c6',
      _darkBlack: '{colors.dark.gray.700}',
      _darkCharcoal: '#323236',
      _darkGraphite: '#383840',
      _darkMidnight: '#282840',
      _darkNavy: '#182838',
      _darkObsidian: '#342840',
      _darkStorm: '#202c38',
      _darkEspresso: '#342820',
    },
  },
  'border.strong': {
    value: {
      base: '{colors.gray.300}',
      _lightWhite: '{colors.gray.300}',
      _lightSnow: '#b8c0d0',
      _lightButter: '#ccc4a8',
      _lightPeach: '#d4a898',
      _lightRose: '#c4a8b8',
      _lightLavender: '#b0acc8',
      _lightMint: '#a4c8b8',
      _lightLatte: '#bcb8a8',
      _darkBlack: '{colors.dark.gray.600}',
      _darkCharcoal: '#424248',
      _darkGraphite: '#48484e',
      _darkMidnight: '#383858',
      _darkNavy: '#243848',
      _darkObsidian: '#443858',
      _darkStorm: '#2c3c4c',
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
      _lightRose: '#e6d8e2',
      _lightLavender: '#d8d8ec',
      _lightMint: '#d8ece2',
      _lightLatte: '#e0dcd2',
      _darkBlack: '{colors.dark.gray.800}',
      _darkCharcoal: '#242428',
      _darkGraphite: '#2c2c30',
      _darkMidnight: '#1c1c30',
      _darkNavy: '#101c28',
      _darkObsidian: '#281c30',
      _darkStorm: '#182028',
      _darkEspresso: '#281c14',
    },
  },
  'border.brand': { value: { base: '{colors.brand.600}', _dark: '{colors.dark.brand.400}' } },
  'border.danger': { value: { base: '{colors.red.600}', _dark: '{colors.dark.red.400}' } },

  'shadow.default': { value: { base: '{colors.gray.950}', _dark: '{colors.dark.gray.950}' } },

  'control.scrollbar.default': {
    value: {
      base: '{colors.gray.200}',
      _lightWhite: '{colors.gray.200}',
      _lightSnow: '#c8ced8',
      _lightButter: '#d4ceb8',
      _lightPeach: '#dab8ac',
      _lightRose: '#ccb8c8',
      _lightLavender: '#bcbacc',
      _lightMint: '#b8d0c4',
      _lightLatte: '#c8c4b4',
      _darkBlack: '{colors.dark.gray.600}',
      _darkCharcoal: '#48484c',
      _darkGraphite: '#4c4c52',
      _darkMidnight: '#3c3c5c',
      _darkNavy: '#2c3c50',
      _darkObsidian: '#4c3c5c',
      _darkStorm: '#3c4850',
      _darkEspresso: '#4c4038',
    },
  },
  'control.scrollbar.hover': {
    value: {
      base: '{colors.gray.300}',
      _lightWhite: '{colors.gray.300}',
      _lightSnow: '#a8b0c0',
      _lightButter: '#beb898',
      _lightPeach: '#c89888',
      _lightRose: '#b49cac',
      _lightLavender: '#a09cb8',
      _lightMint: '#94b8a8',
      _lightLatte: '#aca898',
      _darkBlack: '{colors.dark.gray.500}',
      _darkCharcoal: '#58585c',
      _darkGraphite: '#5c5c62',
      _darkMidnight: '#4c4c6c',
      _darkNavy: '#3c4c60',
      _darkObsidian: '#5c4c6c',
      _darkStorm: '#4c5860',
      _darkEspresso: '#5c5048',
    },
  },

  'decoration.grid.default': { value: { base: '{colors.gray.100}', _dark: '{colors.dark.gray.700}' } },
  'decoration.grid.subtle': { value: { base: '{colors.gray.50}', _dark: '{colors.dark.gray.800}' } },
  'decoration.grid.brand': { value: { base: '{colors.brand.100}', _dark: '{colors.dark.gray.700}' } },
  'decoration.grid.brand.subtle': { value: { base: '{colors.brand.50}', _dark: '{colors.dark.gray.800}' } },

  'callout.info': { value: { base: '#3b82f6', _dark: '#4c6ef5' } },
  'callout.success': { value: { base: '#22c55e', _dark: '#3fc380' } },
  'callout.warning': { value: { base: '#f97316', _dark: '#f4a934' } },
  'callout.danger': { value: { base: '#dc2626', _dark: '#f04444' } },

  'prosemirror.black': {
    value: {
      base: '{colors.gray.900}',
      _lightWhite: '{colors.gray.900}',
      _lightSnow: '#1a2638',
      _lightButter: '#33301a',
      _lightPeach: '#3c2018',
      _lightRose: '#3a1828',
      _lightLavender: '#241840',
      _lightMint: '#1a3028',
      _lightLatte: '#302418',
      _darkBlack: '{colors.dark.gray.50}',
      _darkCharcoal: '#e4e4e6',
      _darkGraphite: '#e8e8ea',
      _darkMidnight: '#dce0f4',
      _darkNavy: '#d4e0ec',
      _darkObsidian: '#e8dcf4',
      _darkStorm: '#d4e0ec',
      _darkEspresso: '#f0e4d8',
    },
  },
  'prosemirror.darkgray': {
    value: {
      base: '#525254',
      _lightWhite: '#525254',
      _lightSnow: '#525b69',
      _lightButter: '#666452',
      _lightPeach: '#6d564f',
      _lightRose: '#6b505d',
      _lightLavender: '#59506f',
      _lightMint: '#52635d',
      _lightLatte: '#63594f',
      _darkBlack: '#b0b0b1',
      _darkCharcoal: '#b2b2b4',
      _darkGraphite: '#b7b7b9',
      _darkMidnight: '#aaadbf',
      _darkNavy: '#a2adb9',
      _darkObsidian: '#b5aabf',
      _darkStorm: '#a5afba',
      _darkEspresso: '#bbb1a6',
    },
  },
  'prosemirror.gray': {
    value: {
      base: '#8c8c8d',
      _lightWhite: '#8c8c8d',
      _lightSnow: '#89909a',
      _lightButter: '#999789',
      _lightPeach: '#9e8c86',
      _lightRose: '#9c8891',
      _lightLavender: '#8e889e',
      _lightMint: '#8a9791',
      _lightLatte: '#968f86',
      _darkBlack: '#7b7b7c',
      _darkCharcoal: '#7f7f81',
      _darkGraphite: '#858588',
      _darkMidnight: '#787a89',
      _darkNavy: '#707a86',
      _darkObsidian: '#81798a',
      _darkStorm: '#757e87',
      _darkEspresso: '#867d74',
    },
  },
  'prosemirror.lightgray': {
    value: {
      base: '#c5c5c6',
      _lightWhite: '#c5c5c6',
      _lightSnow: '#c1c4ca',
      _lightButter: '#cccbc1',
      _lightPeach: '#cec2bd',
      _lightRose: '#cdc0c6',
      _lightLavender: '#c3bfcd',
      _lightMint: '#c1cac6',
      _lightLatte: '#c8c4bd',
      _darkBlack: '#474747',
      _darkCharcoal: '#4d4d4f',
      _darkGraphite: '#545457',
      _darkMidnight: '#464754',
      _darkNavy: '#3e4753',
      _darkObsidian: '#4e4755',
      _darkStorm: '#464d55',
      _darkEspresso: '#514a42',
    },
  },
  'prosemirror.white': {
    value: {
      base: '{colors.white}',
      _lightWhite: '{colors.white}',
      _lightSnow: '#f8f9fb',
      _lightButter: '#fffef8',
      _lightPeach: '#fff8f4',
      _lightRose: '#fef8fa',
      _lightLavender: '#f8f7fc',
      _lightMint: '#f9fdfa',
      _lightLatte: '#fbf9f4',
      _darkBlack: '#121212',
      _darkCharcoal: '#1a1a1c',
      _darkGraphite: '#222226',
      _darkMidnight: '#14141e',
      _darkNavy: '#0c1420',
      _darkObsidian: '#1a1520',
      _darkStorm: '#161c22',
      _darkEspresso: '#1c1610',
    },
  },
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
