<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import type { BlockNode, InlineNode } from './lib/markdown.ts';

  type Props = {
    blocks: BlockNode[];
    plain?: number;
  };

  let { blocks, plain = Number.MAX_SAFE_INTEGER }: Props = $props();

  const wordFade = css({
    opacity: '0',
    animation: '[reveal 0.3s cubic-bezier(0.23, 1, 0.32, 1) forwards]',
    _motionReduce: { opacity: '100', animation: 'none' },
  });

  const codespan = css({
    paddingX: '4px',
    paddingY: '1px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    fontFamily: 'mono',
    fontSize: '13px',
  });

  const headingStyle = (depth: number) =>
    css({
      fontSize: depth <= 2 ? '16px' : depth === 3 ? '15px' : '14px',
      fontWeight: 'semibold',
      lineHeight: '[1.5]',
    });
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
      <em>{@render inline(node.children)}</em>
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
    {:else if node.kind === 'codespan'}
      <code class={codespan}>{node.text}</code>
    {:else if node.kind === 'br'}
      <br />
    {/if}
  {/each}
{/snippet}

<div class={flex({ flexDirection: 'column', gap: '10px', fontSize: '14px', lineHeight: '[1.7]', color: 'text.default' })}>
  {#each blocks as block (block.key)}
    {#if block.kind === 'paragraph'}
      <p>{@render inline(block.children)}</p>
    {:else if block.kind === 'heading'}
      <div class={headingStyle(block.depth)} aria-level={block.depth} role="heading">{@render inline(block.children)}</div>
    {:else if block.kind === 'list'}
      <svelte:element
        this={block.ordered ? 'ol' : 'ul'}
        class={css({
          paddingLeft: '20px',
          display: 'flex',
          flexDirection: 'column',
          gap: '2px',
          listStyleType: block.ordered ? 'decimal' : 'disc',
        })}
        start={block.ordered && block.startIndex !== 1 ? block.startIndex : undefined}
      >
        {#each block.items as item (item.key)}
          <li>
            <PrismMarkdown blocks={item.blocks} {plain} />
          </li>
        {/each}
      </svelte:element>
    {:else if block.kind === 'code'}
      <pre
        class={css({
          padding: '10px',
          borderRadius: '6px',
          backgroundColor: 'surface.muted',
          fontFamily: 'mono',
          fontSize: '13px',
          lineHeight: '[1.6]',
          overflowX: 'auto',
          whiteSpace: 'pre',
        })}><code>{block.text}</code></pre>
    {:else if block.kind === 'blockquote'}
      <blockquote class={css({ borderLeftWidth: '2px', borderColor: 'border.strong', paddingLeft: '10px', color: 'text.subtle' })}>
        <PrismMarkdown blocks={block.children} {plain} />
      </blockquote>
    {:else if block.kind === 'hr'}
      <hr class={css({ borderColor: 'border.subtle' })} />
    {/if}
  {/each}
</div>
