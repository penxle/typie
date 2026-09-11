import { Marked } from 'marked';
import markedCjkFriendly from 'marked-cjk-friendly';
import { markedDirectives } from './directives';
import type { Token, Tokens } from 'marked';
import type { ContainerToken, SpaceToken } from './directives';

export type InlineNode =
  | { kind: 'text'; text: string }
  | { kind: 'strong' | 'em' | 'del'; children: InlineNode[] }
  | { kind: 'link'; href: string; children: InlineNode[] }
  | { kind: 'image'; src: string; alt: string }
  | { kind: 'codespan'; text: string }
  | { kind: 'br' };

export type CellAlign = 'left' | 'center' | 'right' | null;

export type ListItemNode = { task: boolean; checked: boolean; blocks: BlockNode[] };

export type TableCellNode = { align: CellAlign; children: InlineNode[] };

export type TableRowNode = { cells: TableCellNode[] };

export type BlockNode =
  | { kind: 'paragraph'; children: InlineNode[] }
  | { kind: 'heading'; depth: number; children: InlineNode[] }
  | { kind: 'list'; ordered: boolean; startIndex: number; items: ListItemNode[] }
  | { kind: 'table'; header: TableRowNode; rows: TableRowNode[] }
  | { kind: 'code'; text: string }
  | { kind: 'blockquote'; children: BlockNode[] }
  | { kind: 'details'; summary: string; children: BlockNode[] }
  | { kind: 'note'; children: BlockNode[] }
  | { kind: 'space'; lines: number }
  | { kind: 'hr' };

const inlineNodes = (tokens: Token[]): InlineNode[] => {
  const out: InlineNode[] = [];

  for (const token of tokens) {
    switch (token.type) {
      case 'text': {
        const text = token as Tokens.Text;
        if (text.tokens && text.tokens.length > 0) out.push(...inlineNodes(text.tokens));
        else out.push({ kind: 'text', text: text.text });
        break;
      }
      case 'escape': {
        out.push({ kind: 'text', text: (token as Tokens.Escape).text });
        break;
      }
      case 'strong':
      case 'em':
      case 'del': {
        out.push({ kind: token.type, children: inlineNodes((token as Tokens.Strong).tokens) });
        break;
      }
      case 'link': {
        const link = token as Tokens.Link;
        out.push({ kind: 'link', href: link.href, children: inlineNodes(link.tokens) });
        break;
      }
      case 'image': {
        const image = token as Tokens.Image;
        out.push({ kind: 'image', src: image.href, alt: image.text });
        break;
      }
      case 'codespan': {
        out.push({ kind: 'codespan', text: (token as Tokens.Codespan).text });
        break;
      }
      case 'br': {
        out.push({ kind: 'br' });
        break;
      }
      default: {
        out.push({ kind: 'text', text: token.raw });
      }
    }
  }

  return out;
};

const blockNodes = (tokens: Token[]): BlockNode[] => {
  const out: BlockNode[] = [];

  for (const token of tokens) {
    switch (token.type) {
      case 'space':
      case 'def':
      case 'checkbox': {
        break;
      }
      case 'paragraph': {
        out.push({ kind: 'paragraph', children: inlineNodes((token as Tokens.Paragraph).tokens) });
        break;
      }
      case 'text': {
        const text = token as Tokens.Text;
        out.push({ kind: 'paragraph', children: text.tokens ? inlineNodes(text.tokens) : [{ kind: 'text', text: text.text }] });
        break;
      }
      case 'heading': {
        const heading = token as Tokens.Heading;
        out.push({ kind: 'heading', depth: heading.depth, children: inlineNodes(heading.tokens) });
        break;
      }
      case 'list': {
        const list = token as Tokens.List;
        out.push({
          kind: 'list',
          ordered: list.ordered,
          startIndex: typeof list.start === 'number' ? list.start : 1,
          items: list.items.map((item) => ({
            task: item.task === true,
            checked: item.checked === true,
            blocks: blockNodes(item.tokens),
          })),
        });
        break;
      }
      case 'table': {
        const table = token as Tokens.Table;
        const cell = (source: Tokens.TableCell): TableCellNode => ({ align: source.align ?? null, children: inlineNodes(source.tokens) });
        const row = (cells: Tokens.TableCell[]): TableRowNode => ({ cells: cells.map((source) => cell(source)) });
        out.push({ kind: 'table', header: row(table.header), rows: table.rows.map((cells) => row(cells)) });
        break;
      }
      case 'code': {
        out.push({ kind: 'code', text: (token as Tokens.Code).text });
        break;
      }
      case 'blockquote': {
        out.push({ kind: 'blockquote', children: blockNodes((token as Tokens.Blockquote).tokens) });
        break;
      }
      case 'details': {
        const details = token as ContainerToken;
        out.push({ kind: 'details', summary: details.label, children: blockNodes(details.tokens) });
        break;
      }
      case 'note': {
        out.push({ kind: 'note', children: blockNodes((token as ContainerToken).tokens) });
        break;
      }
      case 'blockSpace': {
        out.push({ kind: 'space', lines: (token as SpaceToken).lines });
        break;
      }
      case 'hr': {
        out.push({ kind: 'hr' });
        break;
      }
      default: {
        out.push({ kind: 'paragraph', children: [{ kind: 'text', text: token.raw }] });
      }
    }
  }

  return out;
};

const marked = new Marked({ gfm: true, breaks: true }, markedCjkFriendly(), markedDirectives());

export const parseMarkdown = (source: string): BlockNode[] => {
  if (source.length === 0) return [];
  return blockNodes(marked.lexer(source));
};
