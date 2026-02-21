import { remark } from 'remark';
import remarkGfm from 'remark-gfm';
import { match } from 'ts-pattern';
import type { Nodes } from 'mdast';

const renderInline = (nodes: Nodes[]): string => {
  const parts: { text: string; formatted: boolean }[] = nodes.map((node) =>
    match(node)
      .with({ type: 'text' }, (n) => ({ text: n.value, formatted: false }))
      .with({ type: 'strong' }, (n) => ({ text: `*${renderInline(n.children)}*`, formatted: true }))
      .with({ type: 'emphasis' }, (n) => ({ text: `_${renderInline(n.children)}_`, formatted: true }))
      .with({ type: 'delete' }, (n) => ({ text: `~${renderInline(n.children)}~`, formatted: true }))
      .with({ type: 'inlineCode' }, (n) => ({ text: `\`${n.value}\``, formatted: false }))
      .with({ type: 'link' }, (n) => ({ text: `<${n.url}|${renderInline(n.children)}>`, formatted: false }))
      .with({ type: 'image' }, (n) => ({ text: `<${n.url}>`, formatted: false }))
      .with({ type: 'break' }, () => ({ text: '\n', formatted: false }))
      .otherwise((n) => ({ text: 'children' in n ? renderInline(n.children as Nodes[]) : '', formatted: false })),
  );

  let result = '';
  for (let i = 0; i < parts.length; i++) {
    const { text, formatted } = parts[i];
    if (formatted && result.length > 0 && !/[\s([]$/.test(result)) {
      result += ' ';
    }
    result += text;
    if (formatted && i < parts.length - 1) {
      const next = parts[i + 1];
      if (!/^[\s)\].,!?;:、。]/.test(next.text)) {
        result += ' ';
      }
    }
  }
  return result;
};

const renderTable = (node: Nodes): string => {
  if (node.type !== 'table' || !('children' in node)) return '';
  return (node.children as Nodes[])
    .map((row, i) =>
      match(row)
        .with({ type: 'tableRow' }, (r) => {
          const cells = (r.children as Nodes[]).map((cell) => ('children' in cell ? renderInline(cell.children as Nodes[]) : ''));
          const line = cells.join(' | ');
          return i === 0 ? `*${line}*` : line;
        })
        .otherwise(() => ''),
    )
    .join('\n');
};

const renderBlock = (nodes: Nodes[], indent = ''): string => {
  return nodes
    .map((node) =>
      match(node)
        .with({ type: 'heading' }, (n) => `${indent}*${renderInline(n.children)}*`)
        .with({ type: 'paragraph' }, (n) => `${indent}${renderInline(n.children)}`)
        .with({ type: 'code' }, (n) => `${indent}\`\`\`\n${n.value}\n\`\`\``)
        .with({ type: 'blockquote' }, (n) =>
          renderBlock(n.children as Nodes[])
            .split('\n')
            .map((line) => `${indent}> ${line}`)
            .join('\n'),
        )
        .with({ type: 'list' }, (n) => {
          const nestedIndent = indent + '  ';
          return (n.children as Nodes[])
            .map((item, i) => {
              if (item.type !== 'listItem') return '';
              const prefix = n.ordered ? `${(n.start ?? 1) + i}.` : '•';
              const checked = item.checked;
              const checkbox = checked === true ? '☑ ' : checked === false ? '☐ ' : '';
              const content = (item.children as Nodes[])
                .map((child) =>
                  match(child)
                    .with({ type: 'paragraph' }, (p) => renderInline(p.children))
                    .otherwise((c) => renderBlock([c], nestedIndent)),
                )
                .join('\n');
              return `${indent}${prefix} ${checkbox}${content}`;
            })
            .join('\n');
        })
        .with({ type: 'thematicBreak' }, () => `${indent}──────────`)
        .with({ type: 'html' }, (n) => `${indent}${n.value}`)
        .with({ type: 'table' }, (n) => renderTable(n))
        .otherwise((n) => ('children' in n ? renderBlock(n.children as Nodes[], indent) : '')),
    )
    .join('\n\n');
};

export const toSlackMrkdwn = (markdown: string): string => {
  const tree = remark().use(remarkGfm).parse(markdown);
  return renderBlock(tree.children as Nodes[]);
};
