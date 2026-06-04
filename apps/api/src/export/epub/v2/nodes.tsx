import { Fragment } from 'react';
import { extFromFormat } from '../utils.ts';
import type React from 'react';
import type { Inline, NodeVisitorV2, ParagraphV2, Run } from '../../core/v2/types.ts';

const CALLOUT_ICONS: Record<string, string> = {
  info: 'ℹ️',
  success: '✅',
  warning: '⚠️',
  danger: '🚨',
};

export type EpubConvertContext = Record<string, never>;

export function renderRun(run: Run, key: number): React.ReactNode {
  const { text, style } = run;
  let content: React.ReactNode = text;

  const inlineStyle: React.CSSProperties = {};
  if (style.fontSizePt100) inlineStyle.fontSize = `${style.fontSizePt100 / 100}pt`;
  if (style.fontFamily) inlineStyle.fontFamily = `'${style.fontFamily}'`;
  if (style.fontWeight) inlineStyle.fontWeight = style.fontWeight;
  if (style.textColorHex) inlineStyle.color = `#${style.textColorHex}`;
  if (style.backgroundColorHex) inlineStyle.backgroundColor = `#${style.backgroundColorHex}`;
  if (style.letterSpacing) inlineStyle.letterSpacing = `${style.letterSpacing / 100}em`;

  if (style.bold) content = <strong>{content}</strong>;
  if (style.italic) content = <em>{content}</em>;
  if (style.underline) content = <span style={{ textDecoration: 'underline' }}>{content}</span>;
  if (style.strikethrough) content = <del>{content}</del>;

  if (Object.keys(inlineStyle).length > 0) {
    content = <span style={inlineStyle}>{content}</span>;
  }

  if (style.ruby) {
    content = (
      <ruby>
        {content}
        <rp>(</rp>
        <rt>{style.ruby}</rt>
        <rp>)</rp>
      </ruby>
    );
  }

  if (style.link) {
    content = <a href={/^https?:|^mailto:/i.test(style.link) ? style.link : '#'}>{content}</a>;
  }

  return <Fragment key={key}>{content}</Fragment>;
}

function renderInlineGroup(inlines: Inline[]): React.ReactNode {
  return inlines.map((inline, i) => {
    switch (inline.type) {
      case 'run': {
        return renderRun(inline.run, i);
      }
      case 'hard_break': {
        return <br key={i} />;
      }
      case 'tab': {
        return <Fragment key={i}>{' '}</Fragment>;
      }
    }
  });
}

function renderRuns(runs: Run[]): React.ReactNode {
  return runs.map((run, i) => renderRun(run, i));
}

function renderParagraph(p: ParagraphV2): React.ReactNode {
  const style: React.CSSProperties = {};
  if (p.align && p.align !== 'left') style.textAlign = p.align as React.CSSProperties['textAlign'];
  if (p.lineHeight) style.lineHeight = `${p.lineHeight}%`;

  const groups: Inline[][] = [[]];
  for (const inline of p.inlines) {
    if (inline.type === 'page_break') {
      groups.push([]);
    } else {
      groups.at(-1)?.push(inline);
    }
  }

  if (groups.length === 1) {
    return <p style={Object.keys(style).length > 0 ? style : undefined}>{renderInlineGroup(groups[0])}</p>;
  }

  return (
    <Fragment>
      {groups.map((group, i) => {
        const groupStyle: React.CSSProperties = i < groups.length - 1 ? { ...style, pageBreakAfter: 'always' } : style;
        return (
          <p key={i} style={Object.keys(groupStyle).length > 0 ? groupStyle : undefined}>
            {renderInlineGroup(group)}
          </p>
        );
      })}
    </Fragment>
  );
}

function renderPlaceholder(children: React.ReactNode): React.ReactNode {
  return <p style={{ textAlign: 'center', opacity: 0.5 }}>{children}</p>;
}

function keyed(children: React.ReactNode[]): React.ReactNode {
  return children.map((node, i) => <Fragment key={i}>{node}</Fragment>);
}

export const epubVisitorV2: NodeVisitorV2<EpubConvertContext, React.ReactNode> = {
  paragraph: (p) => renderParagraph(p),

  table: (t) => {
    const hasBorder = t.borderStyle !== 'none';
    const widthPercent = Math.round(t.proportion * 100);

    return (
      <table style={{ ...(hasBorder ? { border: '1px solid #cccccc' } : {}), width: `${widthPercent}%` }}>
        <tbody>
          {t.rows.map((row, ri) => (
            <tr key={ri}>
              {row.cells.map((cell, ci) => {
                const cellStyle: React.CSSProperties = {};
                if (hasBorder) cellStyle.border = '1px solid #cccccc';
                if (cell.colWidth) cellStyle.width = `${cell.colWidth}%`;
                if (cell.backgroundColorHex) cellStyle.backgroundColor = `#${cell.backgroundColorHex}`;

                return (
                  <td key={ci} style={Object.keys(cellStyle).length > 0 ? cellStyle : undefined}>
                    {cell.children.map((node, i) => (
                      <Fragment key={i}>{node}</Fragment>
                    ))}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    );
  },

  image: (n) => {
    const { asset } = n;
    if (asset.width <= 0 || asset.height <= 0) {
      return renderPlaceholder('[이미지를 불러올 수 없습니다]');
    }

    const ext = extFromFormat(asset.format);
    const widthPercent = Math.round(n.proportion * 100);

    return (
      <p style={{ textAlign: 'center' }}>
        <img src={`images/${n.id}.${ext}`} style={{ width: `${widthPercent}%` }} alt="" />
      </p>
    );
  },

  file: () => renderPlaceholder('[파일]'),

  embed: (n) => {
    if (!n.data) return renderPlaceholder('[임베드]');

    const label = n.data.title || n.data.url;
    return renderPlaceholder(<a href={/^https?:|^mailto:/i.test(n.data.url) ? n.data.url : '#'}>{label}</a>);
  },

  archived: () => renderPlaceholder('[보관된 블록]'),

  horizontalRule: () => <hr />,

  bulletList: (items) => (
    <ul>
      {items.map((item, i) => (
        <li key={i}>
          {item.map((node, j) => (
            <Fragment key={j}>{node}</Fragment>
          ))}
        </li>
      ))}
    </ul>
  ),

  orderedList: (items) => (
    <ol>
      {items.map((item, i) => (
        <li key={i}>
          {item.map((node, j) => (
            <Fragment key={j}>{node}</Fragment>
          ))}
        </li>
      ))}
    </ol>
  ),

  blockquote: (variant, children) => {
    switch (variant) {
      case 'left_line': {
        return <blockquote className="left-line">{keyed(children)}</blockquote>;
      }
      case 'left_quote': {
        return (
          <blockquote style={{ marginLeft: 0 }}>
            {'❝ '}
            {keyed(children)}
          </blockquote>
        );
      }
      case 'message_sent': {
        return <blockquote className="message-sent">{keyed(children)}</blockquote>;
      }
      case 'message_received': {
        return <blockquote className="message-received">{keyed(children)}</blockquote>;
      }
      default: {
        return <blockquote>{keyed(children)}</blockquote>;
      }
    }
  },

  callout: (variant, children) => {
    const icon = CALLOUT_ICONS[variant] ?? CALLOUT_ICONS.info;

    return (
      <blockquote className="callout">
        {`${icon} `}
        {keyed(children)}
      </blockquote>
    );
  },

  fold: (title, content) => (
    <blockquote className="fold">
      <details>
        <summary>{renderRuns(title)}</summary>
        {keyed(content)}
      </details>
    </blockquote>
  ),
};
