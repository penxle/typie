import {
  AutoSurround,
  Behavior,
  ClearFormatting,
  Clipboard,
  Command,
  DropCursor,
  IOSSelection,
  NodeCommands,
  NodeId,
  Page,
  Placeholder,
  SafariIMEFix,
  Search,
  Selection,
  SentenceNavigation,
  SyntaxHighlight,
  TrailingNode,
  Typewriter,
  Typography,
} from './extensions';
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
  PageBreak,
  Table,
  TableCell,
  TableRow,
} from './node-views';
import { Body, BulletList, Doc, HardBreak, ListItem, OrderedList, Paragraph, Text } from './nodes';
import { FontFamily, FontSize, FontWeight, TextBackgroundColor, TextColor } from './text-styles';

export const baseExtensions = [
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
  TextBackgroundColor,
  FontFamily,
  FontSize,
  FontWeight,

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
  PageBreak,

  // extensions
  NodeId,
  Page,
  Selection,
  IOSSelection,
  SafariIMEFix,
  SyntaxHighlight,
  TrailingNode,
  Clipboard,
];

export const editorExtensions = [
  // extensions
  AutoSurround,
  Behavior,
  Command,
  ClearFormatting,
  NodeCommands,
  DropCursor,
  Placeholder,
  Search,
  SentenceNavigation,
  Typewriter,
  Typography,

  // menus
  FloatingMenu,
  SlashMenu,
];
