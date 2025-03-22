import { DropCursor, Placeholder, Typography } from './extensions';
import { Bold, FontColor, FontFamily, FontSize, Italic, Ruby, Strike, Underline } from './marks';
import { SlashMenu } from './menus';
import { BulletList, Doc, Document, HardBreak, ListItem, OrderedList, Paragraph, Text } from './nodes';

export const extensions = [
  // special nodes
  Doc,
  Document,
  Text,

  // nodes
  BulletList,
  HardBreak,
  ListItem,
  OrderedList,
  Paragraph,

  // marks
  Bold,
  FontColor,
  FontFamily,
  FontSize,
  Italic,
  Ruby,
  Strike,
  Underline,

  // extensions
  DropCursor,
  Placeholder,
  Typography,

  // menus
  SlashMenu,
];
