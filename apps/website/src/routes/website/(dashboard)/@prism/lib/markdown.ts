import { Lexer } from 'marked';
import type { Token, Tokens } from 'marked';

export type InlineNode =
  | { kind: 'word'; key: number; text: string }
  | { kind: 'space'; key: number; text: string }
  | { kind: 'strong' | 'em' | 'del'; key: number; children: InlineNode[] }
  | { kind: 'link'; key: number; href: string; children: InlineNode[] }
  | { kind: 'codespan'; key: number; text: string }
  | { kind: 'br'; key: number };

export type ListItemNode = { key: number; blocks: BlockNode[] };

export type BlockNode =
  | { kind: 'paragraph'; key: number; children: InlineNode[] }
  | { kind: 'heading'; key: number; depth: number; children: InlineNode[] }
  | { kind: 'list'; key: number; ordered: boolean; startIndex: number; items: ListItemNode[] }
  | { kind: 'code'; key: number; lang: string | null; text: string }
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

const blockNodes = (tokens: Token[], cursor: number): BlockNode[] => {
  const out: BlockNode[] = [];
  for (const token of tokens) {
    switch (token.type) {
      case 'space': {
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
          items.push({ key: itemCursor, blocks: blockNodes(item.tokens, itemCursor) });
          itemCursor += item.raw.length;
        }
        out.push({ kind: 'list', key: cursor, ordered: list.ordered, startIndex: typeof list.start === 'number' ? list.start : 1, items });
        break;
      }
      case 'code': {
        const code = token as Tokens.Code;
        out.push({ kind: 'code', key: cursor, lang: code.lang || null, text: code.text });
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
