<script lang="ts">
  import { EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';

  type Props = {
    characterCount?: number;
    documentCount?: number;
    folderCount?: number;
    loading: boolean;
    visibility: EntityVisibility;
  };

  let { characterCount, documentCount, folderCount, loading, visibility }: Props = $props();
</script>

<div class={flex({ flexDirection: 'column', gap: '4px', minWidth: '120px', opacity: '70' })}>
  <div class={css({ fontWeight: 'semibold' })}>
    {#if visibility === EntityVisibility.PUBLIC}
      <span>공개 폴더</span>
    {:else if visibility === EntityVisibility.UNLISTED}
      <span>링크 조회 가능 폴더</span>
    {:else}
      <span>비공개 폴더</span>
    {/if}
  </div>

  {#if loading}
    <span class={flex({ alignItems: 'center', gap: '4px' })}>
      <RingSpinner style={css.raw({ size: '12px' })} />
      불러오는 중...
    </span>
  {:else if characterCount !== undefined}
    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      {#if folderCount && folderCount > 0}
        <div class={center({ gap: '2px' })}>
          <Icon icon={FolderIcon} size={14} />
          {folderCount}개
        </div>
      {/if}
      {#if documentCount && documentCount > 0}
        <div class={center({ gap: '2px' })}>
          <Icon icon={FileIcon} size={14} />
          {documentCount}개
        </div>
      {/if}
    </div>

    <div>총 {comma(characterCount)}자</div>
  {/if}
</div>
