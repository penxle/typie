import { css } from '@typie/styled-system/css';

export const propertyTriggerStyle = css.raw({
  display: 'inline-flex',
  alignItems: 'center',
  gap: '6px',
  maxWidth: 'full',
  minHeight: '30px',
  marginLeft: '-8px',
  paddingX: '8px',
  paddingY: '4px',
  borderRadius: '6px',
  fontSize: '13px',
  color: 'text.default',
  textAlign: 'left',
  whiteSpace: 'nowrap',
  transition: 'common',
  _hover: { backgroundColor: 'surface.hover' },
  _expanded: { backgroundColor: 'surface.hover' },
  _disabled: { cursor: 'default', _hover: { backgroundColor: 'transparent' } },
});

export const propertyTextStyle = css.raw({
  minWidth: '0',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const propertyHintStyle = css.raw({
  flexShrink: '0',
  fontSize: '12px',
  color: 'text.hint',
});

export const propertyNoteStyle = css.raw({
  marginTop: '-4px',
  paddingLeft: '132px',
  paddingRight: '8px',
  fontSize: '12px',
  lineHeight: '[1.5]',
  color: 'text.hint',
});

export const propertyChevronStyle = css.raw({
  flexShrink: '0',
  color: 'text.hint',
  opacity: '0',
  transition: 'common',
  _groupHover: { opacity: '100' },
  _groupFocusVisible: { opacity: '100' },
  '[aria-expanded="true"] &': { opacity: '100' },
});

export const modifiedBadgeStyle = css.raw({
  flexShrink: '0',
  paddingX: '6px',
  paddingY: '2px',
  borderRadius: '4px',
  fontSize: '10px',
  fontWeight: 'semibold',
  color: 'text.on.success.subtle',
  backgroundColor: 'success.subtle',
});

export const menuListStyle = css.raw({
  gap: '1px',
  paddingX: '4px',
  paddingY: '4px',
});

export const menuItemStyle = css.raw({
  marginX: '0',
  paddingY: '6px',
  color: 'text.default',
});

export const groupLabelStyle = css.raw({
  fontSize: '13px',
  fontWeight: 'semibold',
  color: 'text.muted',
});

export const sectionHeadingStyle = css.raw({
  marginTop: '10px',
  paddingTop: '14px',
  borderTopWidth: '1px',
  borderColor: 'border.hairline',
  fontSize: '12px',
  fontWeight: 'semibold',
  color: 'text.hint',
});

export const tagChipStyle = css.raw({
  display: 'inline-flex',
  alignItems: 'center',
  gap: '4px',
  height: '24px',
  paddingLeft: '9px',
  paddingRight: '5px',
  borderWidth: '1px',
  borderColor: 'border.hairline',
  borderRadius: 'full',
  fontSize: '12px',
  fontWeight: 'medium',
  color: 'text.muted',
  backgroundColor: 'surface.canvas',
});

export const linkFieldStyle = css.raw({
  display: 'flex',
  alignItems: 'center',
  gap: '2px',
  flexShrink: '0',
  width: 'full',
  minWidth: '0',
  height: '32px',
  paddingLeft: '10px',
  paddingRight: '3px',
  borderWidth: '1px',
  borderColor: 'border.hairline',
  borderRadius: '6px',
  backgroundColor: 'surface.canvas',
  transition: 'common',
  _focusWithin: { borderColor: 'border.default' },
});

export const linkFieldInputStyle = css.raw({
  flex: '1',
  minWidth: '0',
  height: 'full',
  fontFamily: 'mono',
  fontSize: '12px',
  color: 'text.muted',
  textOverflow: 'ellipsis',
  cursor: 'pointer',
});

export const linkFieldButtonStyle = css.raw({
  flexShrink: '0',
  size: '26px',
  borderRadius: '4px',
  color: 'text.muted',
  transition: 'common',
  _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
});

export const coverTileStyle = css.raw({
  flexShrink: '0',
  overflow: 'hidden',
  backgroundColor: 'surface.inset',
});
