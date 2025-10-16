<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import mixpanel from 'mixpanel-browser';
  import { goto } from '$app/navigation';
  import { getSplitViewContext, getViewContext } from './context.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  const splitView = getSplitViewContext();
  const view = getViewContext();

  const splitViewId = $derived.by(() => view.id);

  type Props = {
    style?: SystemStyleObject;
    children?: Snippet;
  };

  let { children, style }: Props = $props();

  const splitViewEnabled = $derived(splitView.state.current.enabled);
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
  aria-label={splitViewEnabled ? '스플릿 뷰 닫기' : '닫기'}
  onclick={(e) => {
    e.stopPropagation();

    if (splitViewEnabled) {
      // NOTE: setTimeout을 빼면 마지막 스플릿 뷰를 길게 눌러 닫을 때 unmount가 안 되는 이상한 버그가 있음
      setTimeout(() => {
        const success = splitView.closeSplitView(splitViewId);
        if (!success) return;

        mixpanel.track('close_split_view');
      });
    } else {
      goto('/home');
    }
  }}
  onfocusin={(e) => {
    e.stopPropagation();
  }}
  onkeydown={(e) => {
    e.stopPropagation();
  }}
  type="button"
  use:tooltip={{ message: splitViewEnabled ? '스플릿 뷰 닫기' : '닫기' }}
>
  {@render children?.()}
</button>
