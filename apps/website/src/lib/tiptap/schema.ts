import { Behavior, Command, DropCursor, NodeId, Placeholder, Selection, SyntaxHighlight, TrailingNode, Typography } from './extensions';
import { Bold, Italic, Link, Ruby, Strike, TextStyle, Underline } from './marks';
import { FloatingMenu, SlashMenu } from './menus';
import {
  Blockquote,
  Callout,
  CodeBlock,
  Embed,
  File,
  Fold,
  HorizontalRule,
  HtmlBlock,
  Image,
  Table,
  TableCell,
  TableRow,
} from './node-views';
import { Body, BulletList, Doc, HardBreak, ListItem, OrderedList, Paragraph, Text } from './nodes';
import { FontFamily, FontSize, TextColor } from './text-styles';

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
  Italic,
  Link,
  Ruby,
  Strike,
  TextStyle,
  Underline,

  // text styles
  TextColor,
  FontFamily,
  FontSize,

  // node views
  Blockquote,
  Callout,
  CodeBlock,
  Embed,
  File,
  Fold,
  HorizontalRule,
  HtmlBlock,
  Image,
  Table,
  TableCell,
  TableRow,

  // extensions
  Behavior,
  Command,
  DropCursor,
  NodeId,
  Placeholder,
  Selection,
  SyntaxHighlight,
  TrailingNode,
  Typography,

  // menus
  FloatingMenu,
  SlashMenu,
];
