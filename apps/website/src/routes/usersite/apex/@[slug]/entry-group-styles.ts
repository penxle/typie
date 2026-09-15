import { css } from '@typie/styled-system/css';

export const entryGroupLabel = css.raw({
  display: 'flex',
  alignItems: 'baseline',
  gap: '6px',
  marginBottom: '12px',
  fontSize: '14px',
  fontWeight: 'bold',
  letterSpacing: '-0.01em',
});

export const entryGroupCount = css.raw({ fontSize: '13px', fontWeight: 'medium', color: 'text.hint', fontVariantNumeric: 'tabular-nums' });
