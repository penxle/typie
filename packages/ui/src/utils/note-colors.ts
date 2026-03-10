const textBackgroundColors = [
  { label: '배경 없음', value: 'none', hex: '#ffffff', color: null },
  { label: '그레이', value: 'gray', hex: '#f1f1f2', color: '#f1f1f2' },
  { label: '레드', value: 'red', hex: '#fdebec', color: '#fdebec' },
  { label: '오렌지', value: 'orange', hex: '#ffecd5', color: '#ffecd5' },
  { label: '옐로', value: 'yellow', hex: '#fef3c7', color: '#fef3c7' },
  { label: '그린', value: 'green', hex: '#dff3e3', color: '#dff3e3' },
  { label: '블루', value: 'blue', hex: '#e7f3f8', color: '#e7f3f8' },
  { label: '퍼플', value: 'purple', hex: '#f0e7fe', color: '#f0e7fe' },
] as const;

export const getNoteColors = () => textBackgroundColors.filter((c) => c.value !== 'none');

export const getNoteColor = (value: string) => textBackgroundColors.find((c) => c.value === value);

export const getRandomNoteColor = () => {
  const colors = getNoteColors();
  return colors[Math.floor(Math.random() * colors.length)].value;
};
