import { defineTokens } from '@pandacss/dev';

export { semanticColors } from './semantic-colors.generated';

export const colors = defineTokens.colors({
  current: { value: 'currentColor' },

  white: { value: '#fff' },
  black: { value: '#000' },
  transparent: { value: 'rgb(0 0 0 / 0)' },
});
