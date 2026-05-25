<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import DownloadIcon from '~icons/lucide/download';
  import FileIcon from '~icons/lucide/file';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditorContext } from '../editor.svelte';
  import { createDeleteNodeMessage, deriveFileStage, processFileUpload } from '../handlers/file-flow';
  import { uploadFileAsFile } from '../handlers/upload';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const fileData = $derived(element.data.type === 'file' ? element.data : undefined);
  const fileId = $derived(fileData?.id || undefined);
  const asset = $derived(fileId ? ctx.fileAssets.get(fileId) : undefined);
  const inflight = $derived(ctx.editor?.inflightFiles.get(element.node_id));
  const stage = $derived(deriveFileStage({ fileId, inflight, asset }));

  const canEdit = $derived(!ctx.editor?.readOnly);

  const deleteNode = () => {
    ctx.editor?.enqueue(createDeleteNodeMessage(element.node_id));
    ctx.editor?.focus();
  };

  const handleUpload = () => {
    if (!canEdit) return;

    const picker = document.createElement('input');
    picker.type = 'file';
    picker.multiple = true;

    picker.addEventListener('change', () => {
      const files = [...(picker.files ?? [])];
      if (files.length === 0) return;

      void processFile(files[0]);

      for (const file of files.slice(1)) {
        ctx.pendingFileDrops.push(file);
        ctx.editor?.enqueue({
          type: 'insertion',
          op: { type: 'fragment', fragment: { node: { type: 'file', id: undefined } } },
        });
      }
    });

    picker.click();
  };

  const processFile = async (file: File) => {
    const editor = ctx.editor;
    if (!editor) return;

    const result = await processFileUpload({
      file,
      nodeId: element.node_id,
      setInflightFile: (nodeId, data) => editor.inflightFiles.set(nodeId, data),
      deleteInflightFile: (nodeId) => editor.inflightFiles.delete(nodeId),
      setFileAsset: (a) => ctx.fileAssets.set(a.id, a),
      enqueue: (message) => editor.enqueue(message),
      focus: () => editor.focus(),
      uploadFileAsFile,
    });

    if (result === 'failed') {
      Toast.error(`${file.name} 파일 업로드에 실패했습니다.`);
    }
  };

  $effect(() => {
    if (stage !== 'empty') return;
    const file = ctx.pendingFileDrops.shift();
    if (file) void processFile(file);
  });

  const handleDownload = () => {
    if (!asset) return;
    const a = document.createElement('a');
    a.href = asset.url;
    a.download = asset.name;
    a.click();
  };

  const formatBytes = (bytes: string): string => {
    const n = Number(bytes);
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  };
</script>

<ExternalElementWrapper {element}>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      borderRadius: '4px',
      backgroundColor: 'surface.muted',
      width: 'full',
      minHeight: '48px',
      paddingX: '14px',
      paddingY: '12px',
    })}
  >
    <div class={flex({ align: 'center', gap: '12px', minWidth: '0' })}>
      <Icon class={css({ flexShrink: '0', color: 'text.disabled' })} icon={FileIcon} size={20} />

      <div class={flex({ direction: 'column', gap: '2px', minWidth: '0' })}>
        {#if asset}
          <span
            class={css({
              fontSize: '14px',
              overflow: 'hidden',
              whiteSpace: 'nowrap',
              textOverflow: 'ellipsis',
            })}
          >
            {asset.name}
          </span>
          <span class={css({ fontSize: '12px', color: 'text.disabled' })}>
            {formatBytes(asset.size)}
          </span>
        {:else}
          <span class={css({ fontSize: '14px', color: 'text.disabled' })}>
            {#if stage === 'uploading'}
              {inflight?.name}
            {:else if stage === 'resolving'}
              파일을 불러오는 중...
            {:else}
              파일
            {/if}
          </span>
        {/if}
      </div>
    </div>

    <div class={flex({ align: 'center', gap: '4px', flexShrink: '0' })}>
      {#if stage === 'resolving' || stage === 'uploading'}
        <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
      {:else if asset}
        <button
          class={flex({
            align: 'center',
            gap: '4px',
            borderRadius: '4px',
            paddingX: '8px',
            paddingY: '4px',
            fontSize: '12px',
            color: 'text.muted',
            _hover: { backgroundColor: 'interactive.hover' },
          })}
          aria-label="파일 다운로드"
          onclick={handleDownload}
          onpointerdown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          type="button"
        >
          <Icon icon={DownloadIcon} size={14} />
          다운로드
        </button>
      {:else if canEdit}
        <button
          class={flex({
            align: 'center',
            borderRadius: '4px',
            padding: '4px',
            color: 'text.disabled',
            _hover: { backgroundColor: 'interactive.hover' },
          })}
          aria-label="파일 선택"
          onclick={handleUpload}
          onpointerdown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          type="button"
        >
          <Icon icon={PaperclipIcon} size={16} />
        </button>
      {/if}

      {#if canEdit}
        <button
          class={flex({
            align: 'center',
            borderRadius: '4px',
            padding: '4px',
            color: 'text.disabled',
            _hover: { backgroundColor: 'interactive.hover', color: 'text.danger' },
          })}
          aria-label="파일 삭제"
          onclick={deleteNode}
          onpointerdown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          type="button"
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>
      {/if}
    </div>
  </div>
</ExternalElementWrapper>
