import { NOTE_COLORS } from '@typie/lib/catalogs';
import { token } from '@typie/styled-system/tokens';

export type NoteColorOption = {
  label: string;
  value: string;
  color: string;
};

const noteColorStyles: Record<string, { label: string; color: string }> = {
  gray: { label: '그레이', color: token('colors.palette.gray') },
  red: { label: '레드', color: token('colors.palette.red') },
  orange: { label: '오렌지', color: token('colors.palette.orange') },
  yellow: { label: '옐로', color: token('colors.palette.yellow') },
  green: { label: '그린', color: token('colors.palette.green') },
  blue: { label: '블루', color: token('colors.palette.blue') },
  purple: { label: '퍼플', color: token('colors.palette.purple') },
};

export const noteColors: readonly NoteColorOption[] = NOTE_COLORS.map((value) => ({ ...noteColorStyles[value], value }));

export const getNoteColor = (value: string): string | undefined => noteColors.find((color) => color.value === value)?.color;
