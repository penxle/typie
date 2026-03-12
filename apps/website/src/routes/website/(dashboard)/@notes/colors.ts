import { token } from '@typie/styled-system/tokens';

export const noteColors = [
  { label: '그레이', value: 'gray', color: token('colors.palette.gray') },
  { label: '레드', value: 'red', color: token('colors.palette.red') },
  { label: '오렌지', value: 'orange', color: token('colors.palette.orange') },
  { label: '옐로', value: 'yellow', color: token('colors.palette.yellow') },
  { label: '그린', value: 'green', color: token('colors.palette.green') },
  { label: '블루', value: 'blue', color: token('colors.palette.blue') },
  { label: '퍼플', value: 'purple', color: token('colors.palette.purple') },
];

export const getNoteColor = (value: string) => noteColors.find((c) => c.value === value)?.color;
