<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import SquareIcon from '~icons/lucide/square';
  import SquareCheckIcon from '~icons/lucide/square-check';
  import { markdownCards } from './cards/index.ts';
  import PrismBrokenCard from './cards/PrismBrokenCard.svelte';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import type { BlockNode, CellAlign, InlineNode, TableRowNode } from './lib/markdown.ts';

  type Props = {
    blocks: BlockNode[];
    plain?: number;
    settled?: boolean;
    dense?: boolean;
    depth?: number;
  };

  let { blocks, plain = Number.MAX_SAFE_INTEGER, settled = true, dense = false, depth = 0 }: Props = $props();

  const wordFade = css({
    opacity: '0',
    animation: '[reveal 0.3s cubic-bezier(0.23, 1, 0.32, 1) forwards]',
    _motionReduce: { opacity: '100', animation: 'none' },
  });

  const stack = css({
    display: 'flex',
    flexDirection: 'column',
    fontSize: '14px',
    lineHeight: '[1.7]',
    color: 'text.default',
    overflowWrap: 'anywhere',
  });

  const flowGap = css({ gap: '10px' });
  const denseGap = css({ gap: '6px' });

  const codespan = css({
    paddingX: '4px',
    paddingY: '1px',
    borderRadius: '4px',
    backgroundColor: 'surface.inset',
    fontFamily: 'mono',
    fontSize: '[0.9em]',
  });

  const emphasis = css({ fontStyle: 'italic' });
  const image = css({ display: 'inline-block', verticalAlign: 'middle', maxWidth: 'full', borderRadius: '6px' });

  const headingBase = css({
    marginTop: '10px',
    fontWeight: 'bold',
    lineHeight: '[1.4]',
    _first: { marginTop: '0' },
  });

  const headingSizes = [css({ fontSize: '19px' }), css({ fontSize: '17px' }), css({ fontSize: '15px' }), css({ fontSize: '14px' })];

  const listBase = css({
    display: 'flex',
    flexDirection: 'column',
    gap: '4px',
    paddingLeft: '20px',
  });

  const bulletMarkers = [css({ listStyleType: 'disc' }), css({ listStyleType: 'circle' }), css({ listStyleType: 'square' })];
  const numberMarkers = [css({ listStyleType: 'decimal' }), css({ listStyleType: 'lower-alpha' }), css({ listStyleType: 'lower-roman' })];

  const taskItem = css({ display: 'flex', gap: '6px', listStyleType: 'none' });
  const taskBox = css({ flexShrink: '0', marginTop: '5px', color: 'text.default' });
  const taskBoxChecked = css({ flexShrink: '0', marginTop: '5px', color: 'accent.default' });

  const tableScroll = css({
    overflowX: 'auto',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '6px',
  });

  const table = css({ borderCollapse: 'collapse', width: 'full', fontSize: '13px', lineHeight: '[1.5]' });
  const headCell = css({ paddingX: '10px', paddingY: '6px', fontWeight: 'semibold', backgroundColor: 'surface.canvas' });
  const bodyCell = css({ paddingX: '10px', paddingY: '6px', borderTopWidth: '1px', borderColor: 'border.hairline' });
  const cellDivider = css({ borderLeftWidth: '1px', borderColor: 'border.hairline', _first: { borderLeftWidth: '0' } });

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
  {#each nodes as node (node.key)}
    {#if node.kind === 'word'}
      <span class={node.key >= plain ? wordFade : undefined}>{node.text}</span>
    {:else if node.kind === 'space'}
      {node.text}
    {:else if node.kind === 'strong'}
      <strong class={css({ fontWeight: 'semibold' })}>{@render inline(node.children)}</strong>
    {:else if node.kind === 'em'}
      <em class={emphasis}>{@render inline(node.children)}</em>
    {:else if node.kind === 'del'}
      <del>{@render inline(node.children)}</del>
    {:else if node.kind === 'link'}
      <a
        class={css({ textDecoration: 'underline', textUnderlineOffset: '2px' })}
        href={node.href}
        rel="noopener noreferrer"
        target="_blank"
      >
        {@render inline(node.children)}
      </a>
    {:else if node.kind === 'image'}
      <img class={image} alt={node.alt} src={node.src} />
    {:else if node.kind === 'codespan'}
      <code class={codespan}>{node.text}</code>
    {:else if node.kind === 'br'}
      <br />
    {/if}
  {/each}
{/snippet}

{#snippet cells(row: TableRowNode, head: boolean)}
  {#each row.cells as cell (cell.key)}
    {#if head}
      <th class={cx(headCell, cellDivider, alignOf(cell.align))} scope="col">{@render inline(cell.children)}</th>
    {:else}
      <td class={cx(bodyCell, cellDivider, alignOf(cell.align))}>{@render inline(cell.children)}</td>
    {/if}
  {/each}
{/snippet}

<div class={cx(stack, dense ? denseGap : flowGap)}>
  {#each blocks as block (block.key)}
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
        {#each block.items as item (item.key)}
          <li class={item.task ? taskItem : undefined}>
            {#if item.task}
              <span class={item.checked ? taskBoxChecked : taskBox}>
                <Icon icon={item.checked ? SquareCheckIcon : SquareIcon} size={14} />
              </span>
            {/if}
            <PrismMarkdown blocks={item.blocks} dense depth={depth + 1} {plain} {settled} />
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
            {#each block.rows as row (row.key)}
              <tr>{@render cells(row, false)}</tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if block.kind === 'code'}
      <pre
        class={css({
          paddingY: '10px',
          paddingLeft: '10px',
          borderRadius: '6px',
          backgroundColor: 'surface.inset',
          fontFamily: 'mono',
          fontSize: '13px',
          lineHeight: '[1.6]',
          overflowX: 'auto',
          whiteSpace: 'pre',
        })}><code class={css({ display: 'inline-block', paddingRight: '10px' })}>{block.text}</code></pre>
    {:else if block.kind === 'blockquote'}
      <blockquote class={css({ borderLeftWidth: '2px', borderColor: 'border.emphasis', paddingLeft: '10px', color: 'text.muted' })}>
        <PrismMarkdown blocks={block.children} {dense} {depth} {plain} {settled} />
      </blockquote>
    {:else if block.kind === 'card'}
      {@const Card = Object.hasOwn(markdownCards, block.name) ? markdownCards[block.name] : undefined}
      <div class={block.key >= plain ? wordFade : undefined}>
        {#if Card}
          <Card pending={block.pending} {settled} text={block.text} />
        {:else}
          <PrismBrokenCard />
        {/if}
      </div>
    {:else if block.kind === 'hr'}
      <hr class={css({ flexShrink: '0', height: '1px', backgroundColor: 'border.default' })} />
    {/if}
  {/each}
</div>
