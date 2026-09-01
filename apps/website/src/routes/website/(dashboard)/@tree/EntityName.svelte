<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Marquee } from '@typie/ui/components';

  type Props = {
    name: string;
    active?: boolean;
  };

  let { name, active = false }: Props = $props();

  const getEntityRow = (element: HTMLElement) => element.closest<HTMLElement>('.group, [role="treeitem"]');
</script>

<Marquee
  class={css(
    {
      flexGrow: '1',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'text.muted',
    },
    active && { fontWeight: 'bold', color: 'text.default' },
  )}
  bleed={6}
  fogSize={12}
  getTrigger={getEntityRow}
  text={name}
/>
