import { comma } from '@typie/ui/utils';

export const formatCommaInput = (e: Event): string => {
  const el = e.target as HTMLInputElement;
  const caretDigits = el.value.slice(0, el.selectionStart ?? el.value.length).replaceAll(/\D/g, '').length;
  const digits = el.value.replaceAll(/\D/g, '').slice(0, 15);
  const formatted = digits ? comma(Number(digits)) : '';

  let pos = 0;
  let seen = 0;
  while (pos < formatted.length && seen < caretDigits) {
    if (formatted[pos] !== ',') {
      seen += 1;
    }
    pos += 1;
  }

  el.value = formatted;
  el.setSelectionRange(pos, pos);

  return formatted;
};

export const parseCommaInput = (value: string): number => Number(value.replaceAll(',', ''));
