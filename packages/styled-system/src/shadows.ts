import { defineSemanticTokens } from '@pandacss/dev';

export const semanticShadows = defineSemanticTokens.shadows({
  sm: {
    value: {
      base: ['0 0 2px {colors.shadow.default/4}', '0 1px 4px {colors.shadow.default/3}'],
      _dark: ['0 0 2px {colors.shadow.default/6}', '0 1px 4px {colors.shadow.default/5}'],
    },
  },
  md: {
    value: {
      base: ['0 0 3px {colors.shadow.default/4}', '0 2px 8px {colors.shadow.default/3}'],
      _dark: ['0 0 3px {colors.shadow.default/6}', '0 2px 8px {colors.shadow.default/5}'],
    },
  },
  lg: {
    value: {
      base: ['0 0 4px {colors.shadow.default/4}', '0 4px 16px {colors.shadow.default/3}'],
      _dark: ['0 0 4px {colors.shadow.default/6}', '0 4px 16px {colors.shadow.default/5}'],
    },
  },
  xl: {
    value: {
      base: ['0 0 6px {colors.shadow.default/4}', '0 8px 32px {colors.shadow.default/3}'],
      _dark: ['0 0 6px {colors.shadow.default/6}', '0 8px 32px {colors.shadow.default/5}'],
    },
  },
});
