<script lang="ts">
  import { css, cva } from '$styled-system/css';
  import type { RecipeVariant, SystemStyleObject } from '$styled-system/types';

  type Variants = RecipeVariant<typeof recipe>;

  type Props = {
    color?: Variants['color'];
    style?: SystemStyleObject;
  };

  let { color = 'primary', style }: Props = $props();

  const recipe = cva({
    base: {
      border: 'none',
      width: 'full',
      height: '1px',
      flexShrink: 0,
    },
    variants: {
      color: {
        primary: { backgroundColor: 'interactive.hover' },
        secondary: { backgroundColor: 'surface.muted' },
      },
    },
  });
</script>

<hr class={css(recipe.raw({ color }), style)} />
