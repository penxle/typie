<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { entityIconMap } from '@typie/ui/constants';
  import FileIcon from '~icons/lucide/file';
  import type { DocumentPaneDragGhost } from './document-pane-drag.svelte';

  type Props = {
    ghost: DocumentPaneDragGhost;
  };

  let { ghost }: Props = $props();
</script>

<div
  style:left={`${ghost.x + 8}px`}
  style:top={`${ghost.y}px`}
  style:max-width={`${ghost.width}px`}
  class={flex({
    position: 'fixed',
    alignItems: 'center',
    gap: '2px',
    minWidth: '0',
    paddingX: '8px',
    paddingY: '4px',
    borderRadius: 'full',
    backgroundColor: 'accent.info.default',
    color: 'text.bright',
    fontSize: '14px',
    fontWeight: 'bold',
    pointerEvents: 'none',
    userSelect: 'none',
    zIndex: 'ghost',
  })}
  aria-hidden="true"
  role="presentation"
  use:portal
>
  <Icon style={css.raw({ flexShrink: '0', color: 'text.bright' })} icon={entityIconMap.get(ghost.icon ?? '') ?? FileIcon} size={14} />
  <span class={css({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
    {ghost.name}
  </span>
</div>
