import { cva } from '@typie/styled-system/css';

export const resizeHandle = cva({
  base: {
    touchAction: 'none',
    userSelect: 'none',
    _after: {
      content: '""',
      position: 'absolute',
      borderRadius: 'full',
      backgroundColor: 'border.emphasis',
      pointerEvents: 'none',
      opacity: '0',
    },
    _hoverAfter: { opacity: '100' },
    '&[data-resizing]::after': { opacity: '100' },
  },
  variants: {
    direction: {
      horizontal: {
        cursor: 'col-resize',
        _after: { top: '0', bottom: '0', left: '1/2', width: '2px', transform: 'translateX(-50%)' },
      },
      vertical: {
        cursor: 'row-resize',
        _after: { left: '0', right: '0', top: '1/2', height: '2px', transform: 'translateY(-50%)' },
      },
    },
  },
  defaultVariants: { direction: 'horizontal' },
});
