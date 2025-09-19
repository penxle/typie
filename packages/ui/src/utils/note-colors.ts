import { values } from '@typie/ui/tiptap/values-base';

export const getNoteColors = () => values.textBackgroundColor.filter((c) => c.value !== 'none');

export const getRandomNoteColor = () => {
  const colors = getNoteColors();
  return colors[Math.floor(Math.random() * colors.length)].value;
};
