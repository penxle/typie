import { Behavior, BlockSelectionExt, DropCursor, Placeholder, SyntaxHighlight, Typography } from './extensions';
import { Bold, FontColor, FontFamily, FontSize, Italic, Link, Ruby, Strike, Underline } from './marks';
import { FloatingMenu, SlashMenu } from './menus';
import { Blockquote, Callout, CodeBlock, Embed, File, HorizontalRule, HtmlBlock, Image, Table, TableCell, TableRow } from './node-views';
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
  Link,
  Ruby,
  Strike,
  Underline,

  // node views
  Blockquote,
  Callout,
  CodeBlock,
  Embed,
  File,
  HorizontalRule,
  HtmlBlock,
  Image,
  Table,
  TableCell,
  TableRow,

  // extensions
  Behavior,
  BlockSelectionExt,
  DropCursor,
  Placeholder,
  SyntaxHighlight,
  Typography,

  // menus
  FloatingMenu,
  SlashMenu,
];
