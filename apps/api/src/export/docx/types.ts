import type { PageLayout } from '@/db/schemas/json';
import type { ImageCache } from './nodes/image';
import type { FontMapper } from './utils/font-mapping';

export type ConvertOptions = {
  fontMapper?: FontMapper;
  bodyAttrs?: {
    paragraphIndent?: number;
    blockGap?: number;
  };
  pageLayout?: PageLayout;
  imageCache?: ImageCache;
  depth?: number; // 중첩 깊이
  baseIndent?: number; // 들여쓰기 (twips)
};

export type TextStyles = {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  fontSize?: number; // px
  fontFamily?: string;
  fontWeight?: number;
  color?: string;
  backgroundColor?: string;
  linkHref?: string;
  rubyText?: string;
};

export type Mark = {
  type: string;
  attrs?: {
    fontSize?: string;
    fontFamily?: string;
    textColor?: string;
    textBackgroundColor?: string;
    fontWeight?: number | string;
    href?: string;
    text?: string;
  };
};
