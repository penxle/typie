<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';

  type Suggestion = { name: string; count: number };

  type Props = {
    id: string;
    mine: readonly Suggestion[];
    popular: readonly Suggestion[];
    highlighted: number | null;
    onselect: (name: string) => void;
  };

  let { id, mine, popular, highlighted, onselect }: Props = $props();

  const sections = $derived(
    [
      { title: '내 태그', items: mine, offset: 0 },
      { title: '인기 태그', items: popular, offset: mine.length },
    ].filter((section) => section.items.length > 0),
  );

  const sectionTitleStyle = css.raw({
    paddingX: '10px',
    paddingTop: '6px',
    paddingBottom: '2px',
    fontSize: '11px',
    fontWeight: 'medium',
    color: 'text.hint',
  });

  const optionStyle = css.raw({
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: '12px',
    width: 'full',
    paddingX: '10px',
    paddingY: '5px',
    textAlign: 'left',
    fontSize: '13px',
    color: 'text.default',
    cursor: 'pointer',
    _hover: { backgroundColor: 'surface.hover' },
  });

  const optionActiveStyle = css.raw({
    backgroundColor: 'surface.hover',
  });
</script>

<div
  {id}
  class={flex({
    flexDirection: 'column',
    minWidth: '180px',
    maxWidth: '260px',
    paddingY: '4px',
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    boxShadow: 'lg',
    transformOrigin: 'top left',
    animation: '[drop-in 120ms cubic-bezier(0.16, 1, 0.3, 1)]',
    _dark: { borderWidth: '1px', borderColor: 'border.default' },
    _motionReduce: { animation: '[none]' },
  })}
  aria-label="태그 제안"
  role="listbox"
>
  {#each sections as section (section.title)}
    <div class={css(sectionTitleStyle)} role="presentation">{section.title}</div>

    {#each section.items as item, index (item.name)}
      {@const flatIndex = section.offset + index}
      <button
        id="{id}-{flatIndex}"
        class={css(optionStyle, highlighted === flatIndex ? optionActiveStyle : undefined)}
        aria-selected={highlighted === flatIndex}
        data-name={item.name}
        onclick={() => onselect(item.name)}
        onmousedown={(event) => event.preventDefault()}
        role="option"
        tabindex="-1"
        type="button"
      >
        <span class={css({ overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' })}>{item.name}</span>
        <span class={css({ flexShrink: '0', fontSize: '11px', color: 'text.hint' })}>{item.count}</span>
      </button>
    {/each}
  {/each}
</div>
