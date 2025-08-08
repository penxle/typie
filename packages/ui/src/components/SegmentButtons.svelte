<script generics="T" lang="ts">
  import { css, sva } from '@typie/styled-system/css';
  import type { RecipeVariant } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    value: T;
    items: { label: string; value: T }[];
    size?: Variants['size'];
    onselect?: (value: T) => void;
  };

  let { style, value = $bindable(), items = [], size = 'md', onselect }: Props = $props();

  type Variants = RecipeVariant<typeof recipe>;
  const recipe = sva({
    slots: ['activeIndicator', 'button'],
    base: {
      activeIndicator: {
        position: 'absolute',
        borderRadius: '4px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        transition: '[left 100ms cubic-bezier(0.3, 0, 0, 1)]',
      },
      button: {
        flex: '1',
        zIndex: '1',
        color: 'text.muted',
        _selected: {
          color: 'text.default',
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
  class={css(
    {
      display: 'flex',
      position: 'relative',
      gap: '4px',
      flexShrink: '0',
      padding: '4px',
      borderRadius: '8px',
      backgroundColor: 'surface.muted',
    },
    style,
  )}
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
