import { Behavior, DropCursor, Placeholder, Typography } from './extensions';
import { Bold, FontColor, FontFamily, FontSize, Italic, Ruby, Strike, Underline } from './marks';
import { SlashMenu } from './menus';
import { Body, BulletList, Doc, HardBreak, ListItem, OrderedList, Paragraph, Text } from './nodes';

export const extensions = [
  // special nodes
  Doc,
  Body,
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
  Behavior,
  DropCursor,
  Placeholder,
  Typography,

  // menus
  SlashMenu,
];
