import { Lexer } from 'marked';
import type { Token, Tokens } from 'marked';

export type InlineNode =
  | { kind: 'word'; key: number; text: string }
  | { kind: 'space'; key: number; text: string }
  | { kind: 'strong' | 'em' | 'del'; key: number; children: InlineNode[] }
  | { kind: 'link'; key: number; href: string; children: InlineNode[] }
  | { kind: 'image'; key: number; src: string; alt: string }
  | { kind: 'codespan'; key: number; text: string }
  | { kind: 'br'; key: number };

export type CellAlign = 'left' | 'center' | 'right' | null;

export type ListItemNode = { key: number; task: boolean; checked: boolean; blocks: BlockNode[] };

export type TableCellNode = { key: number; align: CellAlign; children: InlineNode[] };

export type TableRowNode = { key: number; cells: TableCellNode[] };

export type BlockNode =
  | { kind: 'paragraph'; key: number; children: InlineNode[] }
  | { kind: 'heading'; key: number; depth: number; children: InlineNode[] }
  | { kind: 'list'; key: number; ordered: boolean; startIndex: number; items: ListItemNode[] }
  | { kind: 'table'; key: number; header: TableRowNode; rows: TableRowNode[] }
  | { kind: 'code'; key: number; lang: string | null; text: string }
  | { kind: 'card'; key: number; name: string; text: string; pending: boolean }
  | { kind: 'blockquote'; key: number; children: BlockNode[] }
  | { kind: 'hr'; key: number };

const words = (text: string, key: number): InlineNode[] => {
  const out: InlineNode[] = [];
  let start = key;
  for (const part of text.split(/(\s+)/)) {
    if (part.length > 0) out.push({ kind: part.trim().length > 0 ? 'word' : 'space', key: start, text: part });
    start += part.length;
  }
  return out;
};

const inlineNodes = (tokens: Token[], cursor: number): InlineNode[] => {
  const out: InlineNode[] = [];
  for (const token of tokens) {
    switch (token.type) {
      case 'text': {
        const text = token as Tokens.Text;
        if (text.tokens && text.tokens.length > 0) out.push(...inlineNodes(text.tokens, cursor));
        else out.push(...words(text.text, cursor));
        break;
      }
      case 'escape': {
        out.push({ kind: 'word', key: cursor, text: (token as Tokens.Escape).text });
        break;
      }
      case 'strong':
      case 'em':
      case 'del': {
        out.push({ kind: token.type, key: cursor, children: inlineNodes((token as Tokens.Strong).tokens, cursor) });
        break;
      }
      case 'link': {
        out.push({
          kind: 'link',
          key: cursor,
          href: (token as Tokens.Link).href,
          children: inlineNodes((token as Tokens.Link).tokens, cursor),
        });
        break;
      }
      case 'image': {
        const image = token as Tokens.Image;
        out.push({ kind: 'image', key: cursor, src: image.href, alt: image.text });
        break;
      }
      case 'codespan': {
        out.push({ kind: 'codespan', key: cursor, text: (token as Tokens.Codespan).text });
        break;
      }
      case 'br': {
        out.push({ kind: 'br', key: cursor });
        break;
      }
      default: {
        out.push(...words(token.raw, cursor));
      }
    }
    cursor += token.raw.length;
  }
  return out;
};

const CARD_LANG = /^typie:([a-z-]+)$/;

const closedFence = /\n {0,3}(`{3,}|~{3,})[ \t]*$/;
const isFenceClosed = (raw: string): boolean => closedFence.test(raw.replace(/\s+$/, ''));

const blockNodes = (tokens: Token[], cursor: number): BlockNode[] => {
  const out: BlockNode[] = [];
  for (const token of tokens) {
    switch (token.type) {
      case 'space':
      case 'def':
      case 'checkbox': {
        break;
      }
      case 'paragraph': {
        out.push({ kind: 'paragraph', key: cursor, children: inlineNodes((token as Tokens.Paragraph).tokens, cursor) });
        break;
      }
      case 'text': {
        const text = token as Tokens.Text;
        out.push({ kind: 'paragraph', key: cursor, children: text.tokens ? inlineNodes(text.tokens, cursor) : words(text.text, cursor) });
        break;
      }
      case 'heading': {
        const heading = token as Tokens.Heading;
        out.push({ kind: 'heading', key: cursor, depth: heading.depth, children: inlineNodes(heading.tokens, cursor) });
        break;
      }
      case 'list': {
        const list = token as Tokens.List;
        const items: ListItemNode[] = [];
        let itemCursor = cursor;
        for (const item of list.items) {
          items.push({
            key: itemCursor,
            task: item.task === true,
            checked: item.checked === true,
            blocks: blockNodes(item.tokens, itemCursor),
          });
          itemCursor += item.raw.length;
        }
        out.push({ kind: 'list', key: cursor, ordered: list.ordered, startIndex: typeof list.start === 'number' ? list.start : 1, items });
        break;
      }
      case 'table': {
        const table = token as Tokens.Table;
        let cellCursor = cursor;
        const cell = (source: Tokens.TableCell): TableCellNode => {
          const node: TableCellNode = { key: cellCursor, align: source.align ?? null, children: inlineNodes(source.tokens, cellCursor) };
          cellCursor += source.text.length + 1;
          return node;
        };
        const row = (cells: Tokens.TableCell[]): TableRowNode => {
          const key = cellCursor;
          cellCursor += 1;
          return { key, cells: cells.map((source) => cell(source)) };
        };
        out.push({ kind: 'table', key: cursor, header: row(table.header), rows: table.rows.map((cells) => row(cells)) });
        break;
      }
      case 'code': {
        const code = token as Tokens.Code;
        const lang = code.lang || null;
        const kind = lang !== null && code.raw.trimStart().startsWith('```') ? (CARD_LANG.exec(lang)?.[1] ?? null) : null;
        if (kind === null) {
          out.push({ kind: 'code', key: cursor, lang, text: code.text });
        } else {
          out.push({ kind: 'card', key: cursor, name: kind, text: code.text, pending: !isFenceClosed(code.raw) });
        }
        break;
      }
      case 'blockquote': {
        out.push({ kind: 'blockquote', key: cursor, children: blockNodes((token as Tokens.Blockquote).tokens, cursor) });
        break;
      }
      case 'hr': {
        out.push({ kind: 'hr', key: cursor });
        break;
      }
      default: {
        out.push({ kind: 'paragraph', key: cursor, children: words(token.raw, cursor) });
      }
    }
    cursor += token.raw.length;
  }
  return out;
};

export const parseMarkdown = (source: string): BlockNode[] => {
  if (source.length === 0) return [];
  return blockNodes(new Lexer({ gfm: true, breaks: true }).lex(source), 0);
};

const CARD_FENCE_OPENING = '```typie:';

const inOpenCardFence = (head: string): boolean => {
  let fence: 'card' | 'other' | null = null;
  for (const line of head.split('\n')) {
    if (fence === null) {
      const opening = /^ {0,3}(`{3,}|~{3,})(.*)$/.exec(line);
      if (opening) fence = opening[1].startsWith('`') && CARD_LANG.test(opening[2].trim()) ? 'card' : 'other';
    } else if (/^ {0,3}(`{3,}|~{3,})[ \t]*$/.test(line)) {
      fence = null;
    }
  }
  return fence === 'card';
};

export const clampStreamingTail = (text: string): string => {
  const nl = text.lastIndexOf('\n');
  const line = text.slice(nl + 1);
  if (line.length === 0 || !line.startsWith('`')) return text;
  const isPrefix =
    line.length <= CARD_FENCE_OPENING.length
      ? CARD_FENCE_OPENING.startsWith(line)
      : line.startsWith(CARD_FENCE_OPENING) && /^[a-z-]*$/.test(line.slice(CARD_FENCE_OPENING.length));
  if (!isPrefix) return text;
  if (inOpenCardFence(text.slice(0, nl + 1))) return text;
  return text.slice(0, Math.max(0, nl));
};
