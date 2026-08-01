import { css } from '@typie/styled-system/css';

export const adminFilledControl = css.raw({
  borderColor: 'transparent',
  backgroundColor: 'surface.muted',
  borderRadius: '8px',
  minHeight: '32px',
  _hover: { backgroundColor: 'interactive.hover' },
  _expanded: { backgroundColor: 'interactive.hover' },
  '&:has(input:not(:placeholder-shown)), &:has(input[aria-live="polite"])': { borderColor: 'transparent' },
});
