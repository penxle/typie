import CodeIcon from '~icons/lucide/code';
import CodeXmlIcon from '~icons/lucide/code-xml';
import FileUpIcon from '~icons/lucide/file-up';
import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
import ImageIcon from '~icons/lucide/image';
import ListIcon from '~icons/lucide/list';
import ListOrderedIcon from '~icons/lucide/list-ordered';
import MinusIcon from '~icons/lucide/minus';
import PaperclipIcon from '~icons/lucide/paperclip';
import TableIcon from '~icons/lucide/table';
import TextQuoteIcon from '~icons/lucide/text-quote';
import type { Editor, Range } from '@tiptap/core';
import type { MenuItem } from './types';

export const chain = (editor: Editor, range: Range) => {
  return editor.chain().focus().deleteRange(range);
};

export const menuItems: MenuItem[] = [
  {
    id: 'blockquote',
    type: 'blockquote',
    group: 'block',
    name: '인용구',
    keywords: ['blockquote'],
    icon: TextQuoteIcon,
    command: ({ editor, range }) => {
      chain(editor, range).toggleBlockquote().run();
    },
  },
  {
    id: 'horizontal-rule',
    type: 'horizontal_rule',
    group: 'block',
    name: '구분선',
    keywords: ['divider', 'horizontal rule'],
    icon: MinusIcon,
    command: ({ editor, range }) => {
      chain(editor, range).setHorizontalRule().run();
    },
  },
  {
    id: 'bullet-list',
    type: 'bullet_list',
    group: 'block',
    name: '순서 없는 목록',
    keywords: ['bullet list'],
    icon: ListIcon,
    command: ({ editor, range }) => {
      chain(editor, range).toggleBulletList().run();
    },
  },
  {
    id: 'ordered-list',
    type: 'ordered_list',
    group: 'block',
    name: '순서 있는 목록',
    keywords: ['ordered list'],
    icon: ListOrderedIcon,
    command: ({ editor, range }) => {
      chain(editor, range).toggleOrderedList().run();
    },
  },
  {
    id: 'callout',
    type: 'callout',
    group: 'block',
    name: '콜아웃',
    keywords: ['callout'],
    icon: GalleryVerticalEndIcon,
    command: ({ editor, range }) => {
      chain(editor, range).toggleCallout().run();
    },
  },
  {
    id: 'table',
    type: 'table',
    group: 'block',
    name: '표',
    keywords: ['table'],
    icon: TableIcon,
    command: ({ editor, range }) => {
      chain(editor, range).insertTable().run();
    },
  },
  {
    id: 'image',
    type: 'image',
    group: 'media',
    name: '이미지',
    keywords: ['image', 'picture'],
    icon: ImageIcon,
    command: ({ editor, range }) => {
      chain(editor, range).setImage().run();
    },
  },
  {
    id: 'file',
    type: 'file',
    group: 'media',
    name: '파일',
    keywords: ['file', 'attachment'],
    icon: PaperclipIcon,
    command: ({ editor, range }) => {
      chain(editor, range).setFile().run();
    },
  },
  {
    id: 'embed',
    type: 'embed',
    group: 'media',
    name: '임베드',
    keywords: ['embed', 'link'],
    icon: FileUpIcon,
    command: ({ editor, range }) => {
      chain(editor, range).setEmbed().run();
    },
  },
  {
    id: 'code-block',
    type: 'code_block',
    group: 'code',
    name: '코드 블록',
    keywords: ['code'],
    icon: CodeIcon,
    command: ({ editor, range }) => {
      chain(editor, range).setCodeBlock().run();
    },
  },
  {
    id: 'html-block',
    type: 'html_block',
    group: 'code',
    name: 'HTML 블록',
    keywords: ['html'],
    icon: CodeXmlIcon,
    command: ({ editor, range }) => {
      chain(editor, range).setHtmlBlock().run();
    },
  },
];
