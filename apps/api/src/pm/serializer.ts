import type { TextSerializer } from '@tiptap/core';

export const textSerializers: Record<string, TextSerializer> = {
  hard_break: () => '\n',
  code_block: () => '',
  html_block: () => '',
};
