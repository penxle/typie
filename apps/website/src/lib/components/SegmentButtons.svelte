<script generics="T" lang="ts">
  import { css, sva } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { RecipeVariant } from '$styled-system/css';

  type Props = {
    value: T;
    items: { label: string; value: T }[];
    size?: Variants['size'];
    onselect?: (value: T) => void;
  };

  let { value = $bindable(), items = [], size = 'md', onselect }: Props = $props();

  type Variants = RecipeVariant<typeof recipe>;
  const recipe = sva({
    slots: ['activeIndicator', 'button'],
    base: {
      activeIndicator: {
        position: 'absolute',
        borderRadius: '6px',
        background: 'white',
        borderWidth: '1px',
        transition: '[left 100ms cubic-bezier(0.3, 0, 0, 1)]',
      },
      button: {
        flex: '1',
        zIndex: '1',
        color: 'gray.600',
        _selected: {
          color: 'gray.950',
        },
      },
    },
    variants: {
      size: {
        sm: {
          activeIndicator: {
            height: '28px',
          },
          button: {
            fontSize: '12px',
            fontWeight: 'semibold',
            height: '28px',
          },
        },
        md: {
          activeIndicator: {
            height: '36px',
          },
          button: {
            fontSize: '16px',
            fontWeight: 'medium',
            height: '36px',
          },
        },
      },
    },
  });

  const classes = $derived(recipe.raw({ size }));

  const selectedIndex = $derived(items.findIndex((item) => item.value === value));
</script>

<div
  class={flex({
    position: 'relative',
    gap: '4px',
    padding: '4px',
    borderRadius: '10px',
    backgroundColor: 'gray.100',
  })}
>
  <div
    style:left={`calc(4px + ${selectedIndex} * ((100% - ${4 * (items.length + 1)}px) / ${items.length} + 4px))`}
    style:width={`calc((100% - ${4 * (items.length + 1)}px) / ${items.length})`}
    class={css(classes.activeIndicator)}
    aria-hidden="true"
  ></div>

  {#each items as item (item.value)}
    <button
      class={css(classes.button)}
      aria-selected={value === item.value}
      onclick={() => {
        value = item.value;
        onselect?.(value);
      }}
      role="tab"
      type="button"
    >
      {item.label}
    </button>
  {/each}
</div>
