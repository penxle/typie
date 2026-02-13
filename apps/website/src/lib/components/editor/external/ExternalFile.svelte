<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { nanoid } from 'nanoid';
  import DownloadIcon from '~icons/lucide/download';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { uploadBlobAsFile } from '$lib/utils/blob.svelte';
  import { formatFileSize } from '$lib/utils/format';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement, ExternalElementData } from '$lib/editor/types';

  type FileData = Extract<ExternalElementData, { type: 'file' }>;

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const { editor } = getEditorContext();

  let pickerOpened = $state(false);
  let processedUploadId = $state<string>();
  let localUploadId = $state<string>();

  const fileData = $derived(el.data as FileData);
  const isEditable = $derived(!editor.isReadOnly());
  const asset = $derived(fileData.id ? editor.fileAssets.get(fileData.id) : undefined);
  const currentUploadId = $derived(fileData.uploadId ?? localUploadId);
  const inflight = $derived(currentUploadId ? editor.inflightFiles.get(currentUploadId) : undefined);
  const hasFile = $derived(!!asset || !!inflight);
  const isUploading = $derived(!!inflight && !asset);
  const displayName = $derived(asset?.name ?? inflight?.name ?? '파일');
  const displaySize = $derived.by(() => {
    const size = asset?.size ?? inflight?.size;
    return size && size > 0 ? formatFileSize(size) : undefined;
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [hide()],
  });

  $effect(() => {
    pickerOpened = el.isSelected;
  });

  $effect(() => {
    const uploadId = fileData.uploadId;
    if (uploadId && uploadId !== processedUploadId) {
      const file = editor.popUpload(uploadId);
      if (file) {
        processedUploadId = uploadId;
        void processFile(file, uploadId);
      } else {
        console.warn('Upload file not found for uploadId:', uploadId);
      }
    }

    return () => {
      if (uploadId) {
        editor.popUpload(uploadId);
      }
    };
  });

  const processFile = async (file: File, uploadId: string) => {
    editor.inflightFiles.set(uploadId, { url: '', name: file.name, size: file.size });

    try {
      const uploaded = await uploadBlobAsFile(file);
      editor.fileAssets.set(uploaded.id, {
        id: uploaded.id,
        url: uploaded.url,
        name: uploaded.name,
        size: uploaded.size,
      });

      editor.dispatch({
        type: 'setFileId',
        nodeId: el.nodeId,
        fileId: uploaded.id,
      });

      editor.focus();
    } catch (err) {
      console.error('File upload failed:', err);
      Toast.error(`${file.name} 파일 업로드에 실패했습니다.`);
    } finally {
      editor.inflightFiles.delete(uploadId);
      localUploadId = undefined;
    }
  };

  const handleDelete = () => {
    editor.dispatch({ type: 'deleteNode', nodeId: el.nodeId });
    editor.focus();
    editor.scrollIntoView();
  };

  const handleDownload = () => {
    if (!asset?.url) return;

    const a = document.createElement('a');
    a.href = asset.url;
    a.click();
  };

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.multiple = true;

    picker.addEventListener('change', async () => {
      pickerOpened = false;

      const files = picker.files;
      if (!files || files.length === 0) {
        return;
      }

      const [firstFile, ...restFiles] = [...files];
      const firstUploadId = fileData.uploadId ?? nanoid();

      if (!fileData.uploadId) {
        localUploadId = firstUploadId;
        editor.queueUpload(firstUploadId, firstFile);
      }
      void processFile(firstFile, firstUploadId);

      for (const file of restFiles) {
        const uploadId = nanoid();
        editor.queueUpload(uploadId, file);
        try {
          editor.dispatch({
            type: 'insertFile',
            uploadId,
          });
        } catch (err) {
          console.error('Failed to dispatch insertFile:', err);
          editor.popUpload(uploadId);
          Toast.error('파일 업로드에 실패했습니다.');
        }
      }
    });

    picker.click();
  };
</script>

<ExternalElementWrapper {el}>
  <div class={css({ maxWidth: '400px', width: 'full', margin: '[0 auto]' })}>
    {#if hasFile}
      <div
        class={cx(
          'group',
          flex({
            alignItems: 'center',
            gap: '12px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: '8px',
            paddingX: '16px',
            paddingY: '12px',
            backgroundColor: 'surface.muted',
            transition: 'common',
            _hover: { borderColor: 'border.default' },
          }),
        )}
        use:anchor
      >
        <Icon class={css({ color: 'text.muted', flexShrink: '0' })} icon={FileIcon} size={20} />

        <div class={flex({ direction: 'column', flex: '1', minWidth: '0' })}>
          <span
            class={css({
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'text.default',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            })}
          >
            {displayName}
          </span>
          {#if displaySize}
            <span class={css({ fontSize: '12px', color: 'text.muted' })}>
              {displaySize}
            </span>
          {/if}
        </div>

        {#if isEditable}
          <button
            class={css({
              padding: '4px',
              borderRadius: '4px',
              color: 'text.muted',
              opacity: '0',
              transition: 'common',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.danger' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="파일 삭제"
            onclick={handleDelete}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
          </button>
        {/if}

        {#if isUploading}
          <RingSpinner style={css.raw({ size: '20px', color: 'text.disabled' })} />
        {:else if asset?.url}
          <button
            class={css({
              padding: '4px',
              borderRadius: '4px',
              color: 'text.muted',
              transition: 'common',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
            })}
            aria-label="파일 다운로드"
            onclick={handleDownload}
            type="button"
          >
            <Icon icon={DownloadIcon} size={16} />
          </button>
        {/if}
      </div>
    {:else}
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          width: 'full',
          height: '48px',
        })}
        use:anchor
      >
        <div
          class={flex({
            align: 'center',
            gap: '12px',
            paddingX: '14px',
            paddingY: '12px',
            fontSize: '14px',
            color: 'text.disabled',
          })}
        >
          <Icon icon={FileIcon} size={20} />
          파일
        </div>

        {#if isEditable}
          <Menu>
            {#snippet button({ open }: { open: boolean })}
              <div
                class={css(
                  {
                    marginRight: '12px',
                    borderRadius: '4px',
                    padding: '2px',
                    color: 'text.disabled',
                    opacity: '0',
                    transition: 'common',
                    _hover: { backgroundColor: 'interactive.hover' },
                    _groupHover: { opacity: '100' },
                  },
                  open && { opacity: '100' },
                )}
              >
                <Icon icon={EllipsisIcon} size={20} />
              </div>
            {/snippet}

            <MenuItem onclick={handleDelete} variant="danger">
              <Icon icon={Trash2Icon} size={12} />
              <span>삭제</span>
            </MenuItem>
          </Menu>
        {/if}
      </div>
    {/if}
  </div>
</ExternalElementWrapper>

{#if pickerOpened && !hasFile && isEditable}
  <div
    class={center({
      flexDirection: 'column',
      gap: '12px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '380px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      zIndex: 'editor',
    })}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'text.muted' })}>아래 버튼을 클릭해 파일을 선택하세요</span>

    <button
      class={css({
        width: 'full',
        paddingY: '8px',
        paddingX: '16px',
        borderRadius: '6px',
        backgroundColor: 'surface.muted',
        fontSize: '14px',
        transition: 'common',
        _hover: { backgroundColor: 'interactive.hover' },
      })}
      onclick={handleUpload}
      type="button"
    >
      파일 선택
    </button>
  </div>
{/if}
