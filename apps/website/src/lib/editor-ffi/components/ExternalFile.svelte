<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import DownloadIcon from '~icons/lucide/download';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { formatFileSize } from '$lib/utils/format';
  import { getEditorContext } from '../editor.svelte';
  import ExternalCard from './ExternalCard.svelte';
  import ExternalCardAction from './ExternalCardAction.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import ExternalPlaceholder from './ExternalPlaceholder.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const fileData = $derived(element.data.type === 'file' ? element.data : undefined);
  const fileId = $derived(fileData?.id || undefined);
  const asset = $derived(fileId ? ctx.editor?.fileAssets.get(fileId) : undefined);
  const inflight = $derived(ctx.editor?.inflightFiles.get(element.node));
  const stage = $derived.by(() => {
    if (asset) return 'ready';
    if (inflight) return 'uploading';
    if (fileId) return 'resolving';
    return 'empty';
  });

  const canEdit = $derived(!ctx.editor?.readOnly);
  const displayName = $derived(asset?.name || inflight?.name || '파일');
  const downloadOnly = $derived(!canEdit && !!asset);
  const isAttachmentDropTarget = $derived(stage === 'empty' && ctx.attachmentDropTargetNodeId === element.node);

  const deleteNode = () => {
    const editor = ctx.editor;
    if (!editor) return;

    ctx.attachmentImporter.cancelNode(editor, element.node);
    editor.enqueue({
      type: 'node',
      op: { type: 'delete', id: element.node },
    });
    editor.focus();
  };

  const handleUpload = () => {
    const editor = ctx.editor;
    if (!editor || editor.readOnly) return;
    const nodeId = element.node;

    const picker = document.createElement('input');
    picker.type = 'file';
    picker.multiple = true;

    picker.addEventListener('change', () => {
      if (ctx.editor !== editor || editor.destroyed || editor.readOnly) return;
      const items = [...(picker.files ?? [])].map((file) => ({ file, kind: 'file' as const }));
      if (
        ctx.attachmentImporter.importAtSelection(items, {
          existingNodeId: nodeId,
          onFailure: ({ file }) => {
            Toast.error(`${file.name} 파일 업로드에 실패했습니다.`);
          },
        })
      ) {
        editor.focus();
      }
    });

    picker.click();
  };

  const handleDownload = () => {
    if (!asset) return;
    const a = document.createElement('a');
    a.href = asset.url;
    a.download = asset.name;
    a.click();
  };

  const handleCardClick = () => {
    if (ctx.editor?.nativeSelection && !window.getSelection()?.isCollapsed) return;
    handleDownload();
  };
</script>

<ExternalElementWrapper {element}>
  <div class={cx('group', css({ width: 'full' }))}>
    {#if stage === 'ready' || stage === 'uploading'}
      <ExternalCard
        icon={PaperclipIcon}
        label={downloadOnly ? `${displayName} 내려받기` : undefined}
        meta={asset ? formatFileSize(Number(asset.size)) : '업로드 중'}
        onclick={downloadOnly ? handleCardClick : undefined}
        progress={undefined}
        spinner={stage === 'uploading'}
        title={displayName}
      >
        {#if downloadOnly}
          <div
            class={center({
              flexShrink: '0',
              padding: '4px',
              color: 'text.muted',
              transition: 'common',
              _groupHover: { color: 'text.default' },
            })}
            aria-hidden="true"
          >
            <Icon icon={DownloadIcon} size={16} />
          </div>
        {:else}
          <div
            class={flex({
              alignItems: 'center',
              gap: '2px',
              flexShrink: '0',
              opacity: '0',
              transition: 'common',
              _groupActive: { opacity: '100' },
              _groupHover: { opacity: '100' },
              '&:focus-within': { opacity: '100' },
            })}
          >
            {#if asset}
              <ExternalCardAction icon={DownloadIcon} label="파일 내려받기" onclick={handleDownload} />
            {/if}
            {#if canEdit}
              <ExternalCardAction danger icon={Trash2Icon} label="파일 삭제" onclick={deleteNode} />
            {/if}
          </div>
        {/if}
      </ExternalCard>
    {:else if stage === 'resolving'}
      <ExternalCard icon={PaperclipIcon} loading />
    {:else}
      <ExternalPlaceholder
        {canEdit}
        dropActive={isAttachmentDropTarget}
        hint={canEdit ? (isAttachmentDropTarget ? '놓아서 업로드하기' : '클릭하거나 파일을 끌어다 놓으세요') : undefined}
        icon={PaperclipIcon}
        onclick={handleUpload}
        title={canEdit ? '파일 추가' : '비어있는 파일'}
      >
        {#if canEdit}
          <ExternalCardAction danger icon={Trash2Icon} label="파일 삭제" onclick={deleteNode} />
        {/if}
      </ExternalPlaceholder>
    {/if}
  </div>
</ExternalElementWrapper>
