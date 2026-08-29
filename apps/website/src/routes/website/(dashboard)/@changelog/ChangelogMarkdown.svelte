<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import SquareIcon from '~icons/lucide/square';
  import SquareCheckIcon from '~icons/lucide/square-check';
  import ChangelogMarkdown from './ChangelogMarkdown.svelte';
  import type { BlockNode, CellAlign, InlineNode, TableRowNode } from '$lib/markdown/parse';

  type Props = {
    blocks: BlockNode[];
    depth?: number;
  };

  let { blocks, depth = 0 }: Props = $props();

  const stack = css({
    display: 'flex',
    flexDirection: 'column',
    fontSize: '15px',
    lineHeight: '[1.75]',
    overflowWrap: 'anywhere',
  });

  const flowGap = css({ gap: '13px' });
  const nestedGap = css({ gap: '7px' });

  const codespan = css({
    paddingX: '4px',
    paddingY: '1px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    fontFamily: 'mono',
    fontSize: '[0.9em]',
  });

  const image = css({ display: 'block', maxWidth: 'full', height: 'auto', borderRadius: '6px' });

  const headingBase = css({
    marginTop: '11px',
    fontWeight: 'semibold',
    lineHeight: '[1.4]',
    color: 'text.default',
    _first: { marginTop: '0' },
  });

  const headingSizes = [css({ fontSize: '17px' }), css({ fontSize: '16px' }), css({ fontSize: '15px' }), css({ fontSize: '15px' })];

  const listBase = css({ display: 'flex', flexDirection: 'column', gap: '6px', paddingLeft: '22px' });

  const bulletMarkers = [css({ listStyleType: 'disc' }), css({ listStyleType: 'circle' }), css({ listStyleType: 'square' })];
  const numberMarkers = [css({ listStyleType: 'decimal' }), css({ listStyleType: 'lower-alpha' }), css({ listStyleType: 'lower-roman' })];

  const taskItem = css({ display: 'flex', gap: '8px', listStyleType: 'none' });
  const taskBox = css({ flexShrink: '0', marginTop: '6px', color: 'text.faint' });
  const taskBoxChecked = css({ flexShrink: '0', marginTop: '6px', color: 'text.subtle' });

  const tableScroll = css({ overflowX: 'auto', borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '6px' });
  const table = css({ borderCollapse: 'collapse', width: 'full', fontSize: '14px', lineHeight: '[1.6]' });
  const headCell = css({ paddingX: '12px', paddingY: '8px', fontWeight: 'semibold', backgroundColor: 'surface.subtle' });
  const bodyCell = css({ paddingX: '12px', paddingY: '8px', borderTopWidth: '1px', borderColor: 'border.subtle' });
  const cellDivider = css({ borderLeftWidth: '1px', borderColor: 'border.subtle', _first: { borderLeftWidth: '0' } });

  const alignments = {
    left: css({ textAlign: 'left' }),
    center: css({ textAlign: 'center' }),
    right: css({ textAlign: 'right' }),
  };

  const alignOf = (align: CellAlign) => (align === null ? alignments.left : alignments[align]);
  const markerOf = (ordered: boolean) => (ordered ? numberMarkers : bulletMarkers)[depth % 3];
  const headingSizeOf = (level: number) => headingSizes[Math.min(level, headingSizes.length) - 1];
</script>

{#snippet inline(nodes: InlineNode[])}
  {#each nodes as node, idx (idx)}
    {#if node.kind === 'text'}
      {node.text}
    {:else if node.kind === 'strong'}
      <strong class={css({ fontWeight: 'semibold', color: 'text.default' })}>{@render inline(node.children)}</strong>
    {:else if node.kind === 'em'}
      <em class={css({ fontStyle: 'italic' })}>{@render inline(node.children)}</em>
    {:else if node.kind === 'del'}
      <del>{@render inline(node.children)}</del>
    {:else if node.kind === 'link'}
      <a
        class={css({ color: 'text.default', textDecoration: 'underline', textUnderlineOffset: '2px' })}
        href={node.href}
        rel="noopener noreferrer"
        target="_blank"
      >
        {@render inline(node.children)}
      </a>
    {:else if node.kind === 'image'}
      <img class={image} alt={node.alt} loading="lazy" src={node.src} />
    {:else if node.kind === 'codespan'}
      <code class={codespan}>{node.text}</code>
    {:else if node.kind === 'br'}
      <br />
    {/if}
  {/each}
{/snippet}

{#snippet cells(row: TableRowNode, head: boolean)}
  {#each row.cells as cell, idx (idx)}
    {#if head}
      <th class={cx(headCell, cellDivider, alignOf(cell.align))} scope="col">{@render inline(cell.children)}</th>
    {:else}
      <td class={cx(bodyCell, cellDivider, alignOf(cell.align))}>{@render inline(cell.children)}</td>
    {/if}
  {/each}
{/snippet}

<div class={cx(stack, depth > 0 ? nestedGap : flowGap)}>
  {#each blocks as block, idx (idx)}
    {#if block.kind === 'paragraph'}
      <p>{@render inline(block.children)}</p>
    {:else if block.kind === 'heading'}
      <div class={cx(headingBase, headingSizeOf(block.depth))} aria-level={block.depth} role="heading">
        {@render inline(block.children)}
      </div>
    {:else if block.kind === 'list'}
      <svelte:element
        this={block.ordered ? 'ol' : 'ul'}
        class={cx(listBase, markerOf(block.ordered))}
        start={block.ordered && block.startIndex !== 1 ? block.startIndex : undefined}
      >
        {#each block.items as item, itemIdx (itemIdx)}
          <li class={item.task ? taskItem : undefined}>
            {#if item.task}
              <span class={item.checked ? taskBoxChecked : taskBox}>
                <Icon icon={item.checked ? SquareCheckIcon : SquareIcon} size={16} />
              </span>
            {/if}
            <ChangelogMarkdown blocks={item.blocks} depth={depth + 1} />
          </li>
        {/each}
      </svelte:element>
    {:else if block.kind === 'table'}
      <div class={tableScroll}>
        <table class={table}>
          <thead>
            <tr>{@render cells(block.header, true)}</tr>
          </thead>
          <tbody>
            {#each block.rows as row, rowIdx (rowIdx)}
              <tr>{@render cells(row, false)}</tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if block.kind === 'code'}
      <pre
        class={css({
          padding: '12px',
          borderRadius: '6px',
          backgroundColor: 'surface.muted',
          fontFamily: 'mono',
          fontSize: '14px',
          lineHeight: '[1.6]',
          overflowX: 'auto',
          whiteSpace: 'pre',
        })}><code>{block.text}</code></pre>
    {:else if block.kind === 'blockquote'}
      <blockquote class={css({ borderLeftWidth: '2px', borderColor: 'border.strong', paddingLeft: '13px', color: 'text.faint' })}>
        <ChangelogMarkdown blocks={block.children} {depth} />
      </blockquote>
    {:else if block.kind === 'hr'}
      <hr class={css({ flexShrink: '0', height: '1px', backgroundColor: 'border.default', border: 'none' })} />
    {/if}
  {/each}
</div>
