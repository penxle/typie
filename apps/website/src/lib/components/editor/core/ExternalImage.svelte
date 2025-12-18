<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { nanoid } from 'nanoid';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditor } from '$lib/editor/context';
  import { calculateImageDisplaySize } from '$lib/editor/utils';
  import { uploadBlobAsImage } from '$lib/utils/blob.svelte';
  import type { ExternalElement } from '$lib/editor/types';

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const editor = getEditor();

  // TODO: isEditable 구현 필요
  const isEditable = true;

  let inflightUrl = $state<string>();
  let processedUploadId = $state<string>();

  const hasImage = $derived(!!el.data.src || !!inflightUrl);
  const isUploading = $derived(!!inflightUrl);

  let pickerOpened = $state(false);
  $effect(() => {
    pickerOpened = el.isSelected;
  });

  let containerEl = $state<HTMLDivElement>();
  let proportion = $state(el.data.proportion);
  let isResizing = $state(false);
  let initialResizeData: { x: number; width: number; proportion: number; reverse: boolean } | null = null;

  $effect(() => {
    if (!isResizing) {
      proportion = el.data.proportion;
    }
  });

  $effect(() => {
    const uploadId = el.data.uploadId;
    if (uploadId && uploadId !== processedUploadId) {
      const file = editor.popUpload(uploadId);
      if (file) {
        processedUploadId = uploadId;
        void processFile(file);
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

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
  });

  const handleDelete = () => {
    editor.dispatch({ type: 'deleteNode', nodeId: el.nodeId });
    editor.focus();
  };

  const getImageDimensions = (src: string): Promise<{ width: number; height: number }> => {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.addEventListener('load', () => {
        resolve({ width: img.naturalWidth, height: img.naturalHeight });
      });
      img.addEventListener('error', () => {
        reject(new Error('Failed to load image'));
      });
      img.src = src;
    });
  };

  const processFile = async (file: File) => {
    const objectUrl = URL.createObjectURL(file);

    try {
      const { width, height } = await getImageDimensions(objectUrl);

      editor.dispatch({
        type: 'setImageDimensions',
        nodeId: el.nodeId,
        width,
        height,
      });

      inflightUrl = objectUrl;

      const uploadedImage = await uploadBlobAsImage(file);

      editor.dispatch({
        type: 'setImageSrc',
        nodeId: el.nodeId,
        src: uploadedImage.url,
        width,
        height,
      });
      inflightUrl = undefined;
      editor.focus();
    } catch (err) {
      console.error('Image upload failed:', err);
      Toast.error(`${file.name} 이미지 업로드에 실패했습니다.`);
      inflightUrl = undefined;
    } finally {
      if (!inflightUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    }
  };

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = 'image/*';
    picker.multiple = true;

    picker.addEventListener('change', async () => {
      pickerOpened = false;

      const files = picker.files;
      if (!files || files.length === 0) {
        return;
      }

      const [firstFile, ...restFiles] = [...files];

      void processFile(firstFile);

      for (const file of restFiles) {
        const uploadId = nanoid();
        editor.queueUpload(uploadId, file);
        try {
          editor.dispatch({
            type: 'insertImage',
            uploadId,
          });
        } catch (err) {
          console.error('Failed to dispatch insertImage:', err);
          editor.popUpload(uploadId);
          Toast.error('이미지 업로드에 실패했습니다.');
        }
      }
    });

    picker.click();
  };

  const handleResizeStart = (event: PointerEvent, reverse: boolean) => {
    if (!containerEl) return;

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    event.preventDefault();

    isResizing = true;
    initialResizeData = {
      x: event.clientX,
      width: containerEl.clientWidth,
      proportion,
      reverse,
    };
  };

  const handleResize = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId) || !initialResizeData || !containerEl) {
      return;
    }

    const dx = (event.clientX - initialResizeData.x) * (initialResizeData.reverse ? -1 : 1);
    const maxWidth = el.data.originalWidth ?? 0;
    if (maxWidth <= 0) return;

    const newWidth = Math.max(maxWidth * 0.1, Math.min(maxWidth, initialResizeData.width + dx * 2));
    proportion = newWidth / maxWidth;

    editor.dispatch({
      type: 'setImageProportion',
      nodeId: el.nodeId,
      proportion,
    });
  };

  const handleResizeEnd = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture(event.pointerId);

    isResizing = false;

    editor.dispatch({
      type: 'setImageProportion',
      nodeId: el.nodeId,
      proportion,
    });
    editor.focus();
  };

  $effect(() => {
    return () => {
      if (inflightUrl?.startsWith('blob:')) {
        URL.revokeObjectURL(inflightUrl);
      }
    };
  });

  const originalWidth = $derived(el.data.originalWidth ?? 0);
  const originalHeight = $derived(el.data.originalHeight ?? 0);
  const aspectRatio = $derived(originalWidth > 0 ? originalHeight / originalWidth : 0);
  const { displayWidth } = $derived(calculateImageDisplaySize(el.bounds, originalWidth, originalHeight));

  const liveWidth = $derived(isResizing && originalWidth > 0 ? Math.min(originalWidth * proportion, el.bounds.width) : displayWidth);
  const liveHeight = $derived(
    isResizing && originalWidth > 0 ? Math.min(originalWidth * proportion * aspectRatio, el.bounds.height) : el.bounds.height,
  );
</script>

<div
  style:left="{el.bounds.x}px"
  style:top="{el.bounds.y}px"
  style:width="{el.bounds.width}px"
  style:height="{el.bounds.height}px"
  class={css({
    position: 'absolute',
    userSelect: 'none',
    display: 'flex',
    justifyContent: 'center',
    backgroundColor: 'surface.default', // for mix-blend-mode: difference
  })}
  data-external-element
  data-node-id={el.nodeId}
>
  <div
    bind:this={containerEl}
    style:width="{liveWidth}px"
    style:height="{liveHeight}px"
    class={cx('group', css({ position: 'relative' }))}
    use:anchor
  >
    {#if hasImage}
      {#if el.data.src && !isUploading}
        <img
          class={css({ width: 'full', height: 'full', objectFit: 'contain', borderRadius: '4px' })}
          alt="본문 이미지"
          src={el.data.src}
        />
      {:else if inflightUrl}
        <img
          class={css({ width: 'full', height: 'full', objectFit: 'contain', borderRadius: '4px' })}
          alt=""
          onerror={(e) => {
            (e.currentTarget as HTMLImageElement).src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
          }}
          src={inflightUrl}
        />
        <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white/50' })}>
          <RingSpinner style={css.raw({ size: '24px', color: 'text.disabled' })} />
        </div>
      {/if}

      {#if isEditable}
        <button
          class={css({
            position: 'absolute',
            top: '10px',
            right: '10px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            size: '28px',
            color: 'text.bright',
            backgroundColor: '[#363839/70]',
            opacity: '0',
            transition: 'opacity',
            zIndex: '10',
            _hover: { backgroundColor: '[#363839/40]' },
            _groupHover: { opacity: '100' },
          })}
          aria-label="이미지 삭제"
          onclick={handleDelete}
          type="button"
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>

        <div
          class={flex({
            position: 'absolute',
            top: '0',
            bottom: '0',
            left: '10px',
            alignItems: 'center',
            pointerEvents: 'none',
          })}
        >
          <button
            class={css({
              borderRadius: '4px',
              backgroundColor: 'white/50',
              mixBlendMode: 'difference',
              width: '8px',
              height: '1/3',
              maxHeight: '72px',
              cursor: 'col-resize',
              opacity: '0',
              transition: 'opacity',
              zIndex: '10',
              pointerEvents: 'auto',
              _hover: { backgroundColor: 'white/40' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            onpointerdown={(event) => {
              event.preventDefault();
              handleResizeStart(event, true);
            }}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
          ></button>
        </div>

        <div
          class={flex({
            position: 'absolute',
            top: '0',
            bottom: '0',
            right: '10px',
            alignItems: 'center',
            pointerEvents: 'none',
          })}
        >
          <button
            class={css({
              borderRadius: '4px',
              backgroundColor: 'white/50',
              mixBlendMode: 'difference',
              width: '8px',
              height: '1/3',
              maxHeight: '72px',
              cursor: 'col-resize',
              opacity: '0',
              transition: 'opacity',
              zIndex: '10',
              pointerEvents: 'auto',
              _hover: { backgroundColor: 'white/40' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            onpointerdown={(event) => handleResizeStart(event, false)}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
          ></button>
        </div>
      {/if}
    {:else}
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          width: 'full',
          height: 'full',
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
          <Icon icon={ImageIcon} size={20} />
          이미지
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
  {#if el.isSelected}
    <div class={css({ position: 'absolute', inset: '0', backgroundColor: 'selection', pointerEvents: 'none' })}></div>
  {/if}
</div>

{#if pickerOpened && !hasImage && isEditable}
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
    <span class={css({ fontSize: '13px', color: 'text.muted' })}>아래 버튼을 클릭해 이미지를 선택하세요</span>

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
      이미지 선택
    </button>
  </div>
{/if}
