import { token } from '@typie/styled-system/tokens';

const textBackgroundColors = [
  { label: '배경 없음', value: 'none', hex: '#ffffff', color: null },
  { label: '그레이', value: 'gray', hex: '#f1f1f2', color: token('colors.prosemirror.bg.gray') },
  { label: '레드', value: 'red', hex: '#fdebec', color: token('colors.prosemirror.bg.red') },
  { label: '오렌지', value: 'orange', hex: '#ffecd5', color: token('colors.prosemirror.bg.orange') },
  { label: '옐로', value: 'yellow', hex: '#fef3c7', color: token('colors.prosemirror.bg.yellow') },
  { label: '그린', value: 'green', hex: '#dff3e3', color: token('colors.prosemirror.bg.green') },
  { label: '블루', value: 'blue', hex: '#e7f3f8', color: token('colors.prosemirror.bg.blue') },
  { label: '퍼플', value: 'purple', hex: '#f0e7fe', color: token('colors.prosemirror.bg.purple') },
] as const;

export const getNoteColors = () => textBackgroundColors.filter((c) => c.value !== 'none');

export const getNoteColor = (value: string) => textBackgroundColors.find((c) => c.value === value);

export const getRandomNoteColor = () => {
  const colors = getNoteColors();
  return colors[Math.floor(Math.random() * colors.length)].value;
};
