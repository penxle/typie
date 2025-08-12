<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { textAreaScrollPadding, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import ExpandIcon from '~icons/lucide/expand';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import NotebookTabsIcon from '~icons/lucide/notebook-tabs';
  import { YState } from './state.svelte';
  import type * as Y from 'yjs';

  type Props = {
    doc: Y.Doc;
  };

  let { doc }: Props = $props();

  const app = getAppContext();
  const note = new YState(doc, 'note', '');
</script>

<div class={flex({ flexDirection: 'column', gap: '16px', flexGrow: '1' })}>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center', paddingX: '20px' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={NotebookTabsIcon} size={12} />
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>작성 노트</div>
    </div>

    <button
      class={center({ size: '20px', color: 'text.faint', transition: 'common', _hover: { color: 'text.subtle' } })}
      onclick={() => (app.preference.current.noteExpanded = !app.preference.current.noteExpanded)}
      type="button"
      use:tooltip={{ message: app.preference.current.noteExpanded ? '작게 보기' : '크게 보기' }}
    >
      <Icon icon={app.preference.current.noteExpanded ? Minimize2Icon : ExpandIcon} size={12} />
    </button>
  </div>

  <textarea
    class={css({
      flexGrow: '1',
      width: 'full',
      paddingX: '20px',
      paddingBottom: '20px',
      fontSize: '13px',
      color: 'text.subtle',
      wordBreak: 'break-all',
      resize: 'none',
    })}
    placeholder="포스트에 대해 기억할 내용이나 작성에 도움이 되는 내용이 있다면 자유롭게 적어보세요."
    bind:value={note.current}
    use:textAreaScrollPadding
  ></textarea>
</div>
