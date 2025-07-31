<script lang="ts">
  import { EntityVisibility } from '@/enums';
  import { Checkbox } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';

  type Props = {
    entityId: string;
    visibility?: EntityVisibility;
  };

  let { entityId, visibility }: Props = $props();

  const app = getAppContext();
  const selected = $derived(app.state.tree.selectedEntityIds.has(entityId));

  const handleToggle = (e: MouseEvent) => {
    e.stopPropagation();
    if (e.shiftKey) {
      e.preventDefault();
    } else {
      if (selected) {
        app.state.tree.selectedEntityIds.delete(entityId);
      } else {
        app.state.tree.selectedEntityIds.add(entityId);
        app.state.tree.lastSelectedEntityId = entityId;
      }
    }
  };
</script>

<div class={css({ position: 'relative', flex: 'none', size: '16px' })}>
  <div
    class={css(
      {
        position: 'absolute',
        inset: '0',
        borderRadius: 'full',
        backgroundColor: 'interactive.hover',
        size: '4px',
        margin: 'auto',
        opacity: '100',
        transition: 'common',
        _groupHover: { opacity: '0' },
      },
      visibility === EntityVisibility.UNLISTED && { backgroundColor: 'accent.brand.default' },
      selected && { opacity: '0' },
    )}
  ></div>
  <div
    class={css(
      {
        position: 'absolute',
        inset: '0',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        opacity: '0',
        transition: 'common',
        _groupHover: { opacity: '100' },
      },
      selected && { opacity: '100' },
    )}
  >
    <Checkbox checked={selected} clickPadding={true} onclick={handleToggle} size="sm" />
  </div>
</div>
