import { Fragment } from 'react';
import { resolveColorToHex } from '../core/theme';
import { extFromFormat } from './utils';
import type React from 'react';
import type { NodeVisitor } from '../core/traverse';
import type { Annotation, InlineSegment, NodeEntry, TextSegment } from '../core/types';

const CALLOUT_ICONS: Record<string, string> = {
  info: '\u2139\uFE0F',
  success: '\u2705',
  warning: '\u26A0\uFE0F',
  danger: '\uD83D\uDEA8',
};

export type EpubConvertContext = {
  nodes: Record<string, NodeEntry>;
};

// --- Inline rendering helpers ---

function renderSegment(seg: TextSegment, key: number): React.ReactNode {
  let content: React.ReactNode = seg.text;

  const style: React.CSSProperties = {};
  let hasBold = false;
  let hasItalic = false;
  let hasUnderline = false;
  let hasStrikethrough = false;

  for (const s of seg.styles ?? []) {
    switch (s.type) {
      case 'bold': {
        hasBold = true;
        break;
      }
      case 'italic': {
        hasItalic = true;
        break;
      }
      case 'underline': {
        hasUnderline = true;
        break;
      }
      case 'strikethrough': {
        hasStrikethrough = true;
        break;
      }
      case 'font_size': {
        style.fontSize = `${s.size / 100}pt`;
        break;
      }
      case 'font_family': {
        style.fontFamily = `'${s.family}'`;
        break;
      }
      case 'font_weight': {
        style.fontWeight = s.weight;
        break;
      }
      case 'text_color': {
        const hex = resolveColorToHex(`text.${s.color}`);
        if (hex) style.color = `#${hex}`;
        break;
      }
      case 'background_color': {
        const hex = resolveColorToHex(`bg.${s.color}`);
        if (hex) style.backgroundColor = `#${hex}`;
        break;
      }
      case 'letter_spacing': {
        style.letterSpacing = `${s.spacing / 100}em`;
        break;
      }
    }
  }

  if (hasBold) content = <strong>{content}</strong>;
  if (hasItalic) content = <em>{content}</em>;
  if (hasUnderline) content = <span style={{ textDecoration: 'underline' }}>{content}</span>;
  if (hasStrikethrough) content = <del>{content}</del>;

  if (Object.keys(style).length > 0) {
    content = <span style={style}>{content}</span>;
  }

  const annotations = seg.annotations ?? [];
  const link = annotations.find((a): a is Extract<Annotation, { type: 'link' }> => a.type === 'link');
  const ruby = annotations.find((a): a is Extract<Annotation, { type: 'ruby' }> => a.type === 'ruby');

  if (ruby) {
    content = (
      <ruby>
        {content}
        <rp>(</rp>
        <rt>{ruby.text}</rt>
        <rp>)</rp>
      </ruby>
    );
  }

  if (link) {
    content = <a href={/^https?:|^mailto:/i.test(link.href) ? link.href : '#'}>{content}</a>;
  }

  return <Fragment key={key}>{content}</Fragment>;
}

function renderSegments(segments: InlineSegment[]): React.ReactNode {
  return segments.map((seg, i) => {
    switch (seg.type) {
      case 'text': {
        return renderSegment(seg, i);
      }
      case 'hard_break': {
        return <br key={i} />;
      }
      case 'page_break': {
        return null;
      }
    }
  });
}

/** Render inline content from a raw paragraph entry (for container handlers) */
function renderInlineContent(entry: NodeEntry, ctx: EpubConvertContext): React.ReactNode {
  return (entry.children ?? []).map((childId, i) => {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) return null;

    if (childEntry.type === 'text') {
      const segments = (childEntry.text ?? []) as TextSegment[];
      return <Fragment key={i}>{segments.map((seg, j) => renderSegment(seg, j))}</Fragment>;
    } else if (childEntry.type === 'hard_break') {
      return <br key={i} />;
    }

    return null;
  });
}

function renderParagraphEl(entry: NodeEntry, ctx: EpubConvertContext, opts?: { noIndent?: boolean; prefix?: string }): React.ReactNode {
  const align = entry.align as string | undefined;
  const lineHeight = entry.line_height as number | undefined;
  const style: React.CSSProperties = {};
  if (opts?.noIndent) style.textIndent = 0;
  if (align && align !== 'left') style.textAlign = align as React.CSSProperties['textAlign'];
  if (lineHeight) style.lineHeight = `${lineHeight}%`;

  return (
    <p style={Object.keys(style).length > 0 ? style : undefined}>
      {opts?.prefix}
      {renderInlineContent(entry, ctx)}
    </p>
  );
}

function renderPlaceholder(children: React.ReactNode): React.ReactNode {
  return <p style={{ textAlign: 'center', opacity: 0.5 }}>{children}</p>;
}

// --- Container helper renderers ---

function renderParagraphChildren(entry: NodeEntry, ctx: EpubConvertContext): React.ReactNode {
  return (entry.children ?? []).map((childId) => {
    const childEntry = ctx.nodes[childId];
    if (!childEntry || childEntry.type !== 'paragraph') return null;
    return <Fragment key={childId}>{renderParagraphEl(childEntry, ctx, { noIndent: true })}</Fragment>;
  });
}

function renderPrefixedChildren(entry: NodeEntry, ctx: EpubConvertContext, prefix: string): React.ReactNode {
  let prefixUsed = false;

  return (entry.children ?? []).map((childId) => {
    const childEntry = ctx.nodes[childId];
    if (!childEntry || childEntry.type !== 'paragraph') return null;

    const thisPrefix = prefixUsed ? undefined : prefix;
    prefixUsed = true;
    return <Fragment key={childId}>{renderParagraphEl(childEntry, ctx, { noIndent: true, prefix: thisPrefix })}</Fragment>;
  });
}

// --- Visitor ---

export const epubVisitor: NodeVisitor<EpubConvertContext, React.ReactNode> = {
  paragraph: (node) => {
    const align = node.attrs.align as string | undefined;
    const lineHeight = node.attrs.line_height as number | undefined;
    const style: React.CSSProperties = {};
    if (align && align !== 'left') style.textAlign = align as React.CSSProperties['textAlign'];
    if (lineHeight) style.lineHeight = `${lineHeight}%`;

    return <p style={Object.keys(style).length > 0 ? style : undefined}>{renderSegments(node.segments)}</p>;
  },

  table: (entry, convertChildren, ctx) => {
    const borderStyle = (entry as { border_style?: string }).border_style;
    const hasBorder = borderStyle !== 'none';

    return (
      <table style={hasBorder ? { border: '1px solid #cccccc' } : undefined}>
        <tbody>
          {(entry.children ?? []).map((childId) => {
            const rowEntry = ctx.nodes[childId];
            if (!rowEntry || rowEntry.type !== 'table_row') return null;

            return (
              <tr key={childId}>
                {(rowEntry.children ?? []).map((cellId) => {
                  const cellEntry = ctx.nodes[cellId];
                  if (!cellEntry || cellEntry.type !== 'table_cell') return null;

                  const colWidth = (cellEntry as { col_width?: number | null }).col_width;
                  const cellStyle: React.CSSProperties = {};
                  if (hasBorder) cellStyle.border = '1px solid #cccccc';
                  if (colWidth) cellStyle.width = `${colWidth}%`;

                  const cellChildren = convertChildren(cellEntry);
                  return (
                    <td key={cellId} style={Object.keys(cellStyle).length > 0 ? cellStyle : undefined}>
                      {cellChildren}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    );
  },

  image: (node, asset) => {
    if (asset.width <= 0 || asset.height <= 0) {
      return renderPlaceholder('[이미지를 불러올 수 없습니다]');
    }

    const proportion = (node.attrs.proportion as number) ?? 1;
    const ext = extFromFormat(asset.format);
    const widthPercent = Math.round(proportion * 100);

    return (
      <p style={{ textAlign: 'center' }}>
        <img src={`images/${node.id}.${ext}`} style={{ width: `${widthPercent}%` }} alt="" />
      </p>
    );
  },

  file: () => renderPlaceholder('[파일]'),

  embed: (_id, data) => {
    if (!data) return renderPlaceholder('[임베드]');

    const label = data.title || data.url;
    return renderPlaceholder(<a href={/^https?:|^mailto:/i.test(data.url) ? data.url : '#'}>{label}</a>);
  },

  archived: () => renderPlaceholder('[보관된 블록]'),

  horizontalRule: () => <hr />,

  bulletList: (items) => (
    <ul>
      {items.map((item, i) => (
        <li key={i}>{item}</li>
      ))}
    </ul>
  ),

  orderedList: (items) => (
    <ol>
      {items.map((item, i) => (
        <li key={i}>{item}</li>
      ))}
    </ol>
  ),

  blockquote: (entry, variant, _convertChildren, ctx) => {
    switch (variant) {
      case 'left_line': {
        return <blockquote className="left-line">{renderParagraphChildren(entry, ctx)}</blockquote>;
      }
      case 'left_quote': {
        return <blockquote style={{ marginLeft: 0 }}>{renderPrefixedChildren(entry, ctx, '\u275D\u2002')}</blockquote>;
      }
      case 'message_sent': {
        return <blockquote className="message-sent">{renderParagraphChildren(entry, ctx)}</blockquote>;
      }
      case 'message_received': {
        return <blockquote className="message-received">{renderParagraphChildren(entry, ctx)}</blockquote>;
      }
      default: {
        return <blockquote>{renderParagraphChildren(entry, ctx)}</blockquote>;
      }
    }
  },

  callout: (entry, variant, _convertChildren, ctx) => {
    const icon = CALLOUT_ICONS[variant] ?? CALLOUT_ICONS.info;

    return <blockquote className="callout">{renderPrefixedChildren(entry, ctx, `${icon} `)}</blockquote>;
  },

  fold: (entry, convertChildren, ctx) => {
    let titleEntry: NodeEntry | undefined;
    let contentEntry: NodeEntry | undefined;

    for (const childId of entry.children ?? []) {
      const childEntry = ctx.nodes[childId];
      if (!childEntry) continue;
      if (childEntry.type === 'fold_title') titleEntry = childEntry;
      else if (childEntry.type === 'fold_content') contentEntry = childEntry;
    }

    return (
      <blockquote className="fold">
        <details>
          <summary>{titleEntry && renderInlineContent(titleEntry, ctx)}</summary>
          {contentEntry && convertChildren(contentEntry)}
        </details>
      </blockquote>
    );
  },
};
