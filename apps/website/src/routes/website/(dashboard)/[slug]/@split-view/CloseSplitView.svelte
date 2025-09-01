<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import mixpanel from 'mixpanel-browser';
  import { getSplitViewContext, getViewContext } from './context.svelte';
  import { closeSplitView } from './utils';
  import type { Snippet } from 'svelte';

  const splitView = getSplitViewContext();
  const view = getViewContext();

  const splitViewId = $derived.by(() => view.id);

  type Props = {
    children?: Snippet;
  };

  let { children }: Props = $props();
</script>

<button
  class={center({
    borderRadius: '4px',
    size: '24px',
    color: 'text.faint',
    transition: 'common',
    _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
  })}
  aria-label="스플릿 뷰 닫기"
  onclick={() => {
    // NOTE: setTimeout을 빼면 마지막 스플릿 뷰를 길게 눌러 닫을 때 unmount가 안 되는 이상한 버그가 있음
    setTimeout(() => {
      if (!splitView.state.current.view) return;

      splitView.state.current.view = closeSplitView(splitView.state.current.view, splitViewId);

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [splitViewId]: _removed, ...cleanedCurrentPercentages } = splitView.state.current.currentPercentages;
      splitView.state.current.currentPercentages = cleanedCurrentPercentages;

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [splitViewId]: _removedBase, ...cleanedBasePercentages } = splitView.state.current.basePercentages;
      splitView.state.current.basePercentages = cleanedBasePercentages;

      mixpanel.track('close_split_view');
    });
  }}
  type="button"
  use:tooltip={{ message: '스플릿 뷰 닫기' }}
>
  {@render children?.()}
</button>
