import { css } from '@typie/styled-system/css';

export const entity = css.raw({
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  minWidth: '0',
  _hover: { '& [data-entity-name]': { color: 'text.muted' } },
});

export const entityText = css.raw({ flex: '1', minWidth: '0', display: 'flex', flexDirection: 'column' });
export const entityName = css.raw({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true });
export const entitySub = css.raw({
  fontSize: '12px',
  lineHeight: '[1.45]',
  color: 'text.hint',
  fontVariantNumeric: 'tabular-nums',
  truncate: true,
});
export const description = css.raw({ marginTop: '12px', fontSize: '13px', lineHeight: '[1.6]', color: 'text.muted' });

export const side = css.raw({ display: 'flex', flexDirection: 'column', gap: '4px', minWidth: '0' });
export const sideLabel = css.raw({ display: 'inline-flex', alignItems: 'center', gap: '2px', fontSize: '12px', color: 'text.hint' });
export const sideTitle = css.raw({
  display: 'inline-flex',
  alignItems: 'center',
  gap: '4px',
  maxWidth: 'full',
  fontSize: '14px',
  fontWeight: 'medium',
  lineHeight: '[1.4]',
  transition: 'colors',
  _hover: { color: 'text.muted' },
});
export const sideEmpty = css.raw({ fontSize: '14px', lineHeight: '[1.4]', color: 'text.hint' });
export const truncate = css.raw({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' });
