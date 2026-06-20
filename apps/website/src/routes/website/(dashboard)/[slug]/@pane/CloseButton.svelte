<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import mixpanel from 'mixpanel-browser';
  import { getPane, getPaneGroup } from './context.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  const paneGroup = getPaneGroup();
  const pane = getPane();

  const paneId = $derived.by(() => pane.id);

  type Props = {
    style?: SystemStyleObject;
    children?: Snippet;
  };

  let { children, style }: Props = $props();
</script>

<button
  class={css(
    center.raw({
      borderRadius: '4px',
      size: '24px',
      color: 'text.faint',
      transition: 'common',
      _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
    }),
    style,
  )}
  aria-label="창 닫기"
  onclick={(e) => {
    e.stopPropagation();

    // NOTE: setTimeout을 빼면 마지막 스플릿 뷰를 길게 눌러 닫을 때 unmount가 안 되는 이상한 버그가 있음
    setTimeout(() => {
      const success = paneGroup.removePane(paneId);
      if (!success) return;

      mixpanel.track('close_pane');
    }, 0);
  }}
  onfocusin={(e) => {
    e.stopPropagation();
  }}
  onkeydown={(e) => {
    e.stopPropagation();
  }}
  type="button"
  use:tooltip={{ message: '창 닫기' }}
>
  {@render children?.()}
</button>
