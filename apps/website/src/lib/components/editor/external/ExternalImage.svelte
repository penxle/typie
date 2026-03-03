<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Img, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { nanoid } from 'nanoid';
  import { getContext, untrack } from 'svelte';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { uploadBlobAsImage } from '$lib/utils/blob.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import ExternalImageEnlarge from './ExternalImageEnlarge.svelte';
  import type { ExternalElement, ExternalElementData } from '$lib/editor/types';

  type ImageData = Extract<ExternalElementData, { type: 'image' }>;

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const { editor } = getEditorContext();
  const setTotalBlobSizePlanUpgradeModalOpen = getContext<(() => void) | undefined>('setTotalBlobSizePlanUpgradeModalOpen');

  let pickerOpened = $state(false);
  let proportion = $state(0);
  let isResizing = $state(false);
  let initialResizeData: { x: number; width: number; proportion: number; reverse: boolean } | null = null;
  let processedUploadId = $state<string>();
  let localUploadId = $state<string>();
  let enlarged = $state(false);
  let containerEl = $state<HTMLDivElement>();

  const imageData = $derived(el.data as ImageData);
  const isEditable = $derived(!editor.isReadOnly());
  const asset = $derived(imageData.id ? editor.imageAssets.get(imageData.id) : undefined);
  const currentUploadId = $derived(imageData.uploadId ?? localUploadId);
  const inflight = $derived(currentUploadId ? editor.inflightImages.get(currentUploadId) : undefined);
  const imageSrc = $derived(asset?.url ?? inflight?.url);
  const hasImage = $derived(!!imageSrc);
  const isUploading = $derived(!!inflight && !asset);
  const originalWidth = $derived(asset?.width ?? inflight?.width ?? 0);
  const originalHeight = $derived(asset?.height ?? inflight?.height ?? 0);
  const aspectRatio = $derived(originalWidth > 0 ? originalHeight / originalWidth : 0);

  const liveWidth = $derived(originalWidth <= 0 ? el.bounds.width * proportion : Math.min(originalWidth, el.bounds.width * proportion));
  const liveHeight = $derived(aspectRatio <= 0 ? 0 : liveWidth * aspectRatio);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  $effect(() => {
    pickerOpened = el.isSelected;
  });

  const getWidthBounds = (boundsWidth: number) => {
    const maxWidth = Math.min(originalWidth, boundsWidth);
    const minWidth = Math.max(boundsWidth * 0.1, 100);
    return { minWidth, maxWidth };
  };

  const clampWidth = (width: number, boundsWidth: number) => {
    const { minWidth, maxWidth } = getWidthBounds(boundsWidth);
    return Math.max(minWidth, Math.min(maxWidth, width));
  };

  $effect(() => {
    const dataProportion = imageData.proportion;
    if (!untrack(() => isResizing)) {
      proportion = dataProportion;
    }
  });

  $effect(() => {
    const boundsWidth = el.bounds.width;
    if (isResizing || boundsWidth <= 0 || originalWidth <= 0) return;

    const currentWidth = Math.min(originalWidth, boundsWidth * proportion);
    const clampedWidth = clampWidth(currentWidth, boundsWidth);

    if (currentWidth !== clampedWidth) {
      const newProportion = clampedWidth / boundsWidth;
      proportion = newProportion;
      editor.dispatch({
        type: 'setImageProportion',
        nodeId: el.nodeId,
        proportion: newProportion,
      });
    }
  });

  $effect(() => {
    const uploadId = imageData.uploadId;
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

  $effect(() => {
    if (!hasImage) {
      enlarged = false;
    }
  });

  const getImageDimensions = (src: string): Promise<{ width: number; height: number }> => {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.addEventListener('load', () => resolve({ width: img.naturalWidth, height: img.naturalHeight }));
      img.addEventListener('error', () => reject(new Error('Failed to load image')));
      img.src = src;
    });
  };

  const processFile = async (file: File, uploadId: string) => {
    const objectUrl = URL.createObjectURL(file);

    try {
      const { width, height } = await getImageDimensions(objectUrl);
      editor.inflightImages.set(uploadId, { url: objectUrl, width, height });

      const uploadedImage = await uploadBlobAsImage(file);
      editor.imageAssets.set(uploadedImage.id, {
        id: uploadedImage.id,
        url: uploadedImage.url,
        width: uploadedImage.width,
        height: uploadedImage.height,
        placeholder: uploadedImage.placeholder,
      });

      editor.dispatch({
        type: 'setImageId',
        nodeId: el.nodeId,
        imageId: uploadedImage.id,
      });

      editor.focus();
    } catch (err) {
      console.error('Image upload failed:', err);
      Toast.error(`${file.name} 이미지 업로드에 실패했습니다.`);
    } finally {
      editor.inflightImages.delete(uploadId);
      localUploadId = undefined;
      URL.revokeObjectURL(objectUrl);
    }
  };

  const handleDelete = () => {
    editor.dispatch({ type: 'deleteNode', nodeId: el.nodeId });
    editor.focus();
    editor.scrollIntoView();
  };

  const handleUpload = async () => {
    if (editor.restrictedBlob) {
      setTotalBlobSizePlanUpgradeModalOpen?.();
      return;
    }

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
      const firstUploadId = imageData.uploadId ?? nanoid();

      if (!imageData.uploadId) {
        localUploadId = firstUploadId;
        editor.queueUpload(firstUploadId, firstFile);
      }
      void processFile(firstFile, firstUploadId);

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
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    event.stopPropagation();
    event.preventDefault();

    isResizing = true;
    initialResizeData = {
      x: event.clientX,
      width: liveWidth,
      proportion,
      reverse,
    };
  };

  const handleResize = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId) || !initialResizeData) {
      return;
    }

    const boundsWidth = el.bounds.width;
    if (boundsWidth <= 0) return;

    const dx = (event.clientX - initialResizeData.x) * (initialResizeData.reverse ? -1 : 1);
    const newWidth = clampWidth(initialResizeData.width + dx * 2, boundsWidth);
    proportion = newWidth / boundsWidth;
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
</script>

<ExternalElementWrapper {el} minHeight={hasImage ? undefined : '48px'}>
  <div
    bind:this={containerEl}
    style:width={hasImage ? `${liveWidth}px` : '100%'}
    style:height={hasImage ? `${liveHeight}px` : undefined}
    class={cx('group', css({ position: 'relative', margin: '[0 auto]' }))}
  >
    {#if hasImage}
      <Img
        style={css.raw({ width: 'full', borderRadius: '4px' }, !isEditable && { cursor: 'zoom-in' })}
        alt="본문 이미지"
        aria-label={isEditable ? undefined : '이미지 확대 보기'}
        onclick={() => {
          if (isEditable) {
            return;
          }
          enlarged = true;
        }}
        onkeydown={(event) => {
          if (!isEditable && (event.key === 'Enter' || event.key === ' ')) {
            event.preventDefault();
            enlarged = true;
          }
        }}
        onpointerdown={(e) => {
          if (isEditable) {
            return;
          }
          e.stopPropagation();
        }}
        placeholder={asset?.placeholder}
        progressive
        ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
        role={isEditable ? undefined : 'button'}
        size="full"
        tabindex={isEditable ? undefined : 0}
        url={imageSrc ?? ''}
      />

      {#if isUploading}
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
          onpointerdown={(event) => {
            event.stopPropagation();
          }}
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
</ExternalElementWrapper>

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

{#if enlarged && hasImage && imageSrc}
  <ExternalImageEnlarge
    onclose={() => (enlarged = false)}
    placeholder={asset?.placeholder}
    ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
    referenceEl={containerEl}
    url={imageSrc}
  />
{/if}
