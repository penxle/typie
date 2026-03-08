import { createContext, Fragment, useContext } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { resolveColorToHex } from '../theme';
import { extFromFormat } from './utils';
import type React from 'react';
import type { ImageAsset } from '../external';
import type { Annotation, NodeEntry, TextSegment } from './types';

const CALLOUT_ICONS: Record<string, string> = {
  info: '\u2139\uFE0F',
  success: '\u2705',
  warning: '\u26A0\uFE0F',
  danger: '\uD83D\uDEA8',
};

export type ConvertContext = {
  nodes: Record<string, NodeEntry>;
  assets: Map<string, ImageAsset>;
  embeds: Map<string, { url: string; title: string | null }>;
};

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const EpubContext = createContext<ConvertContext>(null!);

function useEntry(id: string): NodeEntry | undefined {
  return useContext(EpubContext).nodes[id];
}

function useCtx(): ConvertContext {
  return useContext(EpubContext);
}

// --- Node dispatcher ---

function Node({ id }: { id: string }) {
  const entry = useEntry(id);
  if (!entry) return null;

  switch (entry.type) {
    case 'paragraph': {
      return <Paragraph entry={entry} />;
    }
    case 'blockquote': {
      return <Blockquote entry={entry} />;
    }
    case 'callout': {
      return <Callout entry={entry} />;
    }
    case 'bullet_list': {
      return (
        <ul>
          <Children entry={entry} />
        </ul>
      );
    }
    case 'ordered_list': {
      return (
        <ol>
          <Children entry={entry} />
        </ol>
      );
    }
    case 'list_item': {
      return <ListItem entry={entry} />;
    }
    case 'table': {
      return <Table entry={entry} />;
    }
    case 'fold': {
      return <Fold entry={entry} />;
    }
    case 'horizontal_rule': {
      return <hr />;
    }
    case 'page_break': {
      return null;
    }
    case 'image': {
      return <Image entry={entry} />;
    }
    case 'embed': {
      return <Embed entry={entry} />;
    }
    case 'file': {
      return <Placeholder>[파일]</Placeholder>;
    }
    case 'archived': {
      return <Placeholder>[보관된 블록]</Placeholder>;
    }
    default: {
      return null;
    }
  }
}

// --- Block components ---

function Paragraph({ entry, noIndent, prefix }: { entry: NodeEntry; noIndent?: boolean; prefix?: string }) {
  const align = entry.align as string | undefined;
  const lineHeight = entry.line_height as number | undefined;
  const style: React.CSSProperties = {};
  if (noIndent) style.textIndent = 0;
  if (align && align !== 'left') style.textAlign = align as React.CSSProperties['textAlign'];
  if (lineHeight) style.lineHeight = `${lineHeight}%`;

  return (
    <p style={Object.keys(style).length > 0 ? style : undefined}>
      {prefix}
      <InlineContent entry={entry} />
    </p>
  );
}

function Blockquote({ entry }: { entry: NodeEntry }) {
  const variant = (entry as { variant?: string }).variant ?? 'left_line';

  switch (variant) {
    case 'left_line': {
      return (
        <blockquote className="left-line">
          <ParagraphChildren entry={entry} />
        </blockquote>
      );
    }
    case 'left_quote': {
      return (
        <blockquote style={{ marginLeft: 0 }}>
          <PrefixedChildren entry={entry} prefix={'\u275D\u2002'} />
        </blockquote>
      );
    }
    case 'message_sent': {
      return (
        <blockquote className="message-sent">
          <ParagraphChildren entry={entry} />
        </blockquote>
      );
    }
    case 'message_received': {
      return (
        <blockquote className="message-received">
          <ParagraphChildren entry={entry} />
        </blockquote>
      );
    }
    default: {
      return (
        <blockquote>
          <ParagraphChildren entry={entry} />
        </blockquote>
      );
    }
  }
}

function Callout({ entry }: { entry: NodeEntry }) {
  const variant = (entry as { variant?: string }).variant ?? 'info';
  const icon = CALLOUT_ICONS[variant] ?? CALLOUT_ICONS.info;

  return (
    <blockquote className="callout">
      <PrefixedChildren entry={entry} prefix={`${icon} `} />
    </blockquote>
  );
}

function ListItem({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
  const parts: React.ReactNode[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;
    if (childEntry.type === 'paragraph') {
      parts.push(<InlineContent key={childId} entry={childEntry} />);
    } else {
      parts.push(<Node key={childId} id={childId} />);
    }
  }

  return (
    <li>
      {parts.map((part, i) => (
        <Fragment key={i}>
          {i > 0 && <br />}
          {part}
        </Fragment>
      ))}
    </li>
  );
}

function Table({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
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

                return (
                  <td key={cellId} style={Object.keys(cellStyle).length > 0 ? cellStyle : undefined}>
                    <Children entry={cellEntry} />
                  </td>
                );
              })}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function Fold({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
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
        <summary>{titleEntry && <InlineContent entry={titleEntry} />}</summary>
        {contentEntry && <Children entry={contentEntry} />}
      </details>
    </blockquote>
  );
}

function Image({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
  const id = entry.id as string | undefined;
  const proportion = (entry as { proportion?: number }).proportion ?? 1;

  if (!id) return <Placeholder>[이미지]</Placeholder>;

  const asset = ctx.assets.get(id);
  if (!asset) return <Placeholder>[이미지를 불러올 수 없습니다]</Placeholder>;

  const ext = extFromFormat(asset.format);
  const widthPercent = Math.round(proportion * 100);

  return (
    <p style={{ textAlign: 'center' }}>
      <img src={`images/${id}.${ext}`} style={{ width: `${widthPercent}%` }} alt="" />
    </p>
  );
}

function Embed({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
  const embedId = entry.id as string | undefined;
  const embedData = embedId ? ctx.embeds.get(embedId) : undefined;

  if (!embedData) return <Placeholder>[임베드]</Placeholder>;

  const label = embedData.title || embedData.url;
  return (
    <Placeholder>
      <a href={/^https?:|^mailto:/i.test(embedData.url) ? embedData.url : '#'}>{label}</a>
    </Placeholder>
  );
}

function Placeholder({ children }: { children: React.ReactNode }) {
  return <p style={{ textAlign: 'center', opacity: 0.5 }}>{children}</p>;
}

// --- Children renderers ---

function Children({ entry }: { entry: NodeEntry }) {
  return (entry.children ?? []).map((childId) => <Node key={childId} id={childId} />);
}

function ParagraphChildren({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
  return (entry.children ?? []).map((childId) => {
    const childEntry = ctx.nodes[childId];
    if (!childEntry || childEntry.type !== 'paragraph') return null;
    return <Paragraph key={childId} entry={childEntry} noIndent />;
  });
}

function PrefixedChildren({ entry, prefix }: { entry: NodeEntry; prefix: string }) {
  const ctx = useCtx();
  let prefixUsed = false;

  return (entry.children ?? []).map((childId) => {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) return null;

    if (childEntry.type === 'paragraph') {
      const thisPrefix = prefixUsed ? undefined : prefix;
      prefixUsed = true;
      return <Paragraph key={childId} entry={childEntry} noIndent prefix={thisPrefix} />;
    }

    return <Node key={childId} id={childId} />;
  });
}

// --- Inline content ---

function InlineContent({ entry }: { entry: NodeEntry }) {
  const ctx = useCtx();
  return (entry.children ?? []).map((childId, i) => {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) return null;

    if (childEntry.type === 'text') {
      const segments = (childEntry.text as TextSegment[]) ?? [];
      return (
        <Fragment key={i}>
          {segments.map((seg, j) => (
            <Segment key={j} seg={seg} />
          ))}
        </Fragment>
      );
    } else if (childEntry.type === 'hard_break') {
      return <br key={i} />;
    }

    return null;
  });
}

function Segment({ seg }: { seg: TextSegment }) {
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

  return content;
}

// --- Public API ---

export function renderBodyHtml(rootChildren: string[], ctx: ConvertContext): string {
  return renderToStaticMarkup(
    <EpubContext value={ctx}>
      {rootChildren.map((childId) => (
        <Node key={childId} id={childId} />
      ))}
    </EpubContext>,
  );
}
