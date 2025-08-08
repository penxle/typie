<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import type { RecipeVariant, SystemStyleObject } from '@typie/styled-system/types';

  type Variants = RecipeVariant<typeof recipe>;

  type Props = {
    color?: Variants['color'];
    style?: SystemStyleObject;
  };

  let { color = 'primary', style }: Props = $props();

  const recipe = cva({
    base: {
      border: 'none',
      width: '1px',
      height: 'full',
      flexShrink: 0,
    },
    variants: {
      color: {
        primary: { backgroundColor: 'border.default' },
        secondary: { backgroundColor: 'border.strong' },
      },
    },
  });
</script>

<hr class={css(recipe.raw({ color }), style)} />
