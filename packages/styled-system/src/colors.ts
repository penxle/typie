import { defineSemanticTokens, defineTokens } from '@pandacss/dev';

export const colors = defineTokens.colors({
  current: { value: 'currentColor' },

  transparent: { value: 'rgb(0 0 0 / 0)' },

  gray: {
    '50': { value: '#F9FAFB' },
    '100': { value: '#F2F4F6' },
    '150': { value: '#EEEFF1' },
    '200': { value: '#E5E7EA' },
    '300': { value: '#ACB2B9' },
    '400': { value: '#696D72' },
    '500': { value: '#464B53' },
    '600': { value: '#3C4048' },
    '700': { value: '#33363D' },
    '800': { value: '#2A2D32' },
    '850': { value: '#25292D' },
    '900': { value: '#23262A' },
    '950': { value: '#1F2123' },
  },
  primary: {
    '50': { value: '#F7F9FC' },
    '100': { value: '#EEF2F8' },
    '200': { value: '#D6DEFF' },
    '300': { value: '#496AFD' },
    '400': { value: '#415CFB' },
    '500': { value: '#2950FF' },
    '600': { value: '#002BEB' },
    '700': { value: '#0021B3' },
    '800': { value: '#1B2967' },
    '900': { value: '#343B54' },
    '950': { value: '#293240' },
  },
  red: {
    '50': { value: '#FEF5F6' },
    '100': { value: '#FDE1E5' },
    '200': { value: '#FCCED5' },
    '300': { value: '#F78C9C' },
    '400': { value: '#F5667A' },
    '500': { value: '#F2415A' },
    '600': { value: '#E5102E' },
    '700': { value: '#AC0C22' },
    '800': { value: '#570611' },
    '900': { value: '#39040B' },
    '950': { value: '#1D0206' },
  },
  white: { value: '#ffffff' },
});

export const semanticColors = defineSemanticTokens.colors({
  /**
   * Common
   */
  text: {
    primary: { value: { base: '{colors.gray.950}', _dark: '{colors.gray.50}' } },
    secondary: { value: { base: '{colors.gray.500}', _dark: '{colors.gray.200}' } },
    tertiary: { value: { base: '{colors.gray.400}', _dark: '{colors.gray.300}' } },
    quaternary: { value: { base: '{colors.gray.300}', _dark: '{colors.gray.500}' } },
    danger: { value: { base: '{colors.red.600}', _dark: '{colors.red.400}' } },
    emphasis: { value: { base: '{colors.primary.400}', _dark: '{colors.primary.300}' } },
  },
  line: {
    primary: { value: { base: '{colors.gray.100}', _dark: '{colors.gray.850}' } },
    secondary: { value: { base: '{colors.gray.150}', _dark: '{colors.gray.700}' } },
    tertiary: { value: { base: '{colors.gray.200}', _dark: '{colors.gray.600}' } },
    emphasis: { value: { base: '{colors.primary.500}', _dark: '{colors.primary.400}' } },
  },
  background: {
    primary: { value: { base: '{colors.white}', _dark: '{colors.gray.950}' } },
    secondary: { value: { base: '{colors.gray.50}', _dark: '{colors.gray.900}' } },
    tertiary: { value: { base: '{colors.gray.100}', _dark: '{colors.gray.850}' } },
    quaternary: { value: { base: '{colors.gray.150}', _dark: '{colors.gray.800}' } },
    emphasis: { value: { base: '{colors.primary.50}', _dark: '{colors.primary.900}' } },
  },
  overlay: {
    primary: { value: { base: '{colors.white}', _dark: '{colors.gray.900}' } },
    secondary: { value: { base: '{colors.white}', _dark: '{colors.gray.850}' } },
  },
});
