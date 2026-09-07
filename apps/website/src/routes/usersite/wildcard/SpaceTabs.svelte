<script lang="ts" module>
  export type SpaceHomeTab = 'posts' | 'series' | 'tags';
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { onMount, tick } from 'svelte';
  import { getUsersiteChrome } from '../chrome.svelte';
  import { stuck } from './stuck';
  import type { Snippet } from 'svelte';

  type Props = {
    tab: SpaceHomeTab;
    counts: { posts: number; series: number; tags: number };
    onselect: (tab: SpaceHomeTab) => void;
    trailing?: Snippet;
  };

  let { tab, counts, onselect, trailing }: Props = $props();

  const chrome = getUsersiteChrome();

  const items = $derived([
    { id: 'posts', label: '글', count: counts.posts, disabled: false },
    { id: 'series', label: '시리즈', count: counts.series, disabled: counts.series === 0 },
    { id: 'tags', label: '태그', count: counts.tags, disabled: counts.tags === 0 },
  ] as const);

  let container = $state<HTMLDivElement>();
  let buttons = $state<Partial<Record<SpaceHomeTab, HTMLButtonElement>>>({});
  let indicator = $state({ x: 0, width: 0 });
  let ready = $state(false);

  const measure = () => {
    const el = buttons[tab];
    if (!el) return;
    indicator = { x: el.offsetLeft, width: el.offsetWidth };
  };

  $effect(() => {
    void tab;
    measure();
  });

  onMount(() => {
    measure();
    requestAnimationFrame(() => {
      ready = true;
    });

    const observer = new ResizeObserver(measure);
    if (container) observer.observe(container);
    return () => observer.disconnect();
  });

  const select = (next: SpaceHomeTab) => {
    if (items.find((item) => item.id === next)?.disabled) return;
    onselect(next);
  };

  const onkeydown = (e: KeyboardEvent) => {
    const order = items.filter((item) => !item.disabled).map((item) => item.id);
    const index = order.indexOf(tab);
    let next: SpaceHomeTab | undefined;

    if (e.key === 'ArrowRight') next = order[(index + 1) % order.length];
    else if (e.key === 'ArrowLeft') next = order[(index - 1 + order.length) % order.length];
    else if (e.key === 'Home') next = order[0];
    else if (e.key === 'End') next = order.at(-1);
    if (!next) return;

    e.preventDefault();
    e.stopPropagation();
    select(next);
    const target = next;
    void tick().then(() => buttons[target]?.focus());
  };
</script>

<div
  bind:this={container}
  class={css({
    position: 'sticky',
    top: '[var(--usersite-sticky-header-bottom, 0px)]',
    zIndex: '10',
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    borderBottomWidth: '1px',
    borderColor: 'border.hairline',
    backgroundColor: 'surface.default',
    transition: '[box-shadow 150ms ease-out, border-color 150ms ease-out]',
    '&[data-stuck]': { borderColor: 'border.default' },
  })}
  aria-label="스페이스 내용"
  role="tablist"
  use:stuck={{ onchange: (node, value) => chrome.setStuck(node, value) }}
>
  <span
    style:width={`${indicator.width}px`}
    style:transform={`translateX(${indicator.x}px)`}
    class={css(
      { position: 'absolute', left: '0', bottom: '-1px', height: '2px', backgroundColor: 'text.default', willChange: 'transform' },
      ready && { transition: '[transform 200ms cubic-bezier(0.32, 0.72, 0, 1), width 200ms cubic-bezier(0.32, 0.72, 0, 1)]' },
    )}
    aria-hidden="true"
  ></span>

  {#each items as item (item.id)}
    <button
      bind:this={buttons[item.id]}
      id={`space-tab-${item.id}`}
      class={css({
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        height: '44px',
        paddingX: '12px',
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'text.hint',
        transition: 'colors',
        _hover: { color: 'text.muted' },
        _selected: { color: 'text.default' },
        _first: { paddingLeft: '0' },
        _disabled: { color: 'text.hint', opacity: '[0.45]', cursor: 'default' },
      })}
      aria-controls={`space-panel-${item.id}`}
      aria-disabled={item.disabled}
      aria-selected={tab === item.id}
      disabled={item.disabled}
      onclick={() => select(item.id)}
      {onkeydown}
      role="tab"
      tabindex={tab === item.id ? 0 : -1}
      type="button"
    >
      {item.label}
      {#if item.count > 0}
        <span
          class={css({ marginLeft: '5px', fontSize: '12px', fontWeight: 'normal', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}
        >
          {item.count}
        </span>
      {/if}
    </button>
  {/each}

  {#if trailing}
    <div class={css({ marginLeft: 'auto' })}>
      {@render trailing()}
    </div>
  {/if}
</div>
