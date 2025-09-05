import { defaultValues } from '@typie/ui/tiptap/values-base';
import { match } from 'ts-pattern';
import type { Mark, TextStyles } from '../types';
import type { FontMapper } from './font-mapping';

export function processMarks(marks: Mark[], fontMapper?: FontMapper): TextStyles {
  return marks.reduce((acc: TextStyles, mark: Mark) => {
    return match(mark.type)
      .with('bold', () => ({ ...acc, bold: true }))
      .with('italic', () => ({ ...acc, italic: true }))
      .with('underline', () => ({ ...acc, underline: true }))
      .with('strike', () => ({ ...acc, strike: true }))
      .with('text_style', () => {
        const fontFamily = mark.attrs?.fontFamily;
        const fontWeight = mark.attrs?.fontWeight ? Number(mark.attrs.fontWeight) : undefined;
        return {
          ...acc,
          ...(mark.attrs?.fontSize && {
            fontSize: Number.parseInt(mark.attrs.fontSize ?? defaultValues.fontSize),
          }),
          ...(fontFamily && {
            fontFamily: fontMapper?.getFontName(fontFamily, fontWeight) || fontFamily,
          }),
          ...(mark.attrs?.textColor && { color: mark.attrs.textColor }),
          ...(mark.attrs?.textBackgroundColor && { backgroundColor: mark.attrs.textBackgroundColor }),
          ...(fontWeight && { fontWeight }),
        };
      })
      .with('link', () => ({ ...acc, linkHref: mark.attrs?.href || '' }))
      .with('ruby', () => ({ ...acc, rubyText: mark.attrs?.text || '' }))
      .otherwise(() => acc);
  }, {} as TextStyles);
}
