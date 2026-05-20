<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Img, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { nanoid } from 'nanoid';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import ExternalImageEnlarge from '$lib/components/editor/external/ExternalImageEnlarge.svelte';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { uploadBlobAsImage } from '$lib/utils/blob.svelte';
  import { graphql } from '$mearie';
  import { getEditorContext } from '../editor.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const SELECTION_FOCUSED_ALPHA = 77 / 255;
  const SELECTION_UNFOCUSED_ALPHA = 48 / 255;

  const ctx = getEditorContext();
  const theme = getThemeContext();

  const themeVariant = $derived(theme.currentThemeVariant);
  const selectionColor = $derived(THEME_COLORS[themeVariant].selection);
  const selectionOpacity = $derived(ctx.editor?.focused ? SELECTION_FOCUSED_ALPHA : SELECTION_UNFOCUSED_ALPHA);

  let containerEl: HTMLDivElement | null = $state(null);
  let reportedHeight = $state<number>();
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let pickerOpened = $state(false);
  let localUploadId = $state<string>();
  let processedUploadId = $state<string>();
  let proportion = $state(100);
  let isResizing = $state(false);
  let initialResizeData: { x: number; width: number; proportion: number; reverse: boolean } | null = null;
  let enlarged = $state(false);
  let objectUrl: string | undefined;

  const imageData = $derived(element.data.type === 'image' ? element.data : undefined);
  const asset = $derived(imageData?.id ? ctx.editor?.imageAssets.get(imageData.id) : undefined);
  const imageAssetQuery = createQuery(
    graphql(`
      query EditorFfiExternalImage_Query($imageId: ID!) {
        image(imageId: $imageId) {
          id
          url
          originalUrl
          width
          height
          placeholder
        }
      }
    `),
    () => ({ imageId: imageData?.id ?? '' }),
    () => ({ skip: !imageData?.id || !!asset }),
  );
  const currentUploadId = $derived(imageData?.upload_id ?? localUploadId);
  const inflight = $derived(currentUploadId && ctx.editor ? ctx.editor.inflightImages.get(currentUploadId) : undefined);
  const imageSrc = $derived(asset?.url ?? inflight?.url);
  const hasImage = $derived(!!imageSrc);
  const isUploading = $derived(!!inflight && !asset);
  const isResolvingAsset = $derived(!!imageData?.id && !asset && !inflight);
  const originalWidth = $derived(asset?.width ?? inflight?.width ?? 0);
  const originalHeight = $derived(asset?.height ?? inflight?.height ?? 0);
  const aspectRatio = $derived(originalWidth > 0 ? originalHeight / originalWidth : 0);
  const liveWidth = $derived(
    originalWidth <= 0 ? element.bounds.width * (proportion / 100) : Math.min(originalWidth, element.bounds.width * (proportion / 100)),
  );
  const liveHeight = $derived(aspectRatio <= 0 ? 0 : liveWidth * aspectRatio);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  $effect(() => {
    pickerOpened = element.is_selected;
  });

  const getWidthBounds = (boundsWidth: number) => {
    const maxWidth = Math.min(originalWidth || boundsWidth, boundsWidth);
    const minWidth = Math.min(maxWidth, Math.max(boundsWidth * 0.1, 100));
    return { minWidth, maxWidth };
  };

  const clampWidth = (width: number, boundsWidth: number) => {
    const { minWidth, maxWidth } = getWidthBounds(boundsWidth);
    return Math.max(minWidth, Math.min(maxWidth, width));
  };

  const clampProportion = (value: number) => Math.max(10, Math.min(100, Math.round(value)));

  const getImageDimensions = (src: string): Promise<{ width: number; height: number }> => {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.addEventListener('load', () => resolve({ width: img.naturalWidth, height: img.naturalHeight }));
      img.addEventListener('error', () => reject(new Error('Failed to load image')));
      img.src = src;
    });
  };

  const deleteNode = () => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'node', op: { type: 'delete', id: element.node_id } });
    editor.focus();
  };

  const stopPointerPropagation = (event: PointerEvent) => {
    event.stopPropagation();
  };

  const processFile = async (file: File, uploadId: string) => {
    const editor = ctx.editor;
    if (!editor) return;

    objectUrl = URL.createObjectURL(file);

    try {
      const { width, height } = await getImageDimensions(objectUrl);
      editor.inflightImages.set(uploadId, { url: objectUrl, width, height });

      const uploadedImage = await uploadBlobAsImage(file);
      editor.imageAssets.set(uploadedImage.id, {
        id: uploadedImage.id,
        url: uploadedImage.url,
        originalUrl: uploadedImage.originalUrl,
        width: uploadedImage.width,
        height: uploadedImage.height,
        placeholder: uploadedImage.placeholder,
      });

      if (!editor.hasExternalElement(element.node_id)) {
        return;
      }

      editor.enqueue({
        type: 'node',
        op: {
          type: 'set_image_id',
          id: element.node_id,
          image_id: uploadedImage.id,
        },
      });
      editor.focus();
    } catch (err) {
      console.error('Image upload failed:', err);
      Toast.error(`${file.name} 이미지 업로드에 실패했습니다.`);
      if (!imageData?.id && editor.hasExternalElement(element.node_id)) {
        deleteNode();
      }
    } finally {
      editor.clearImageUpload(uploadId);
      localUploadId = undefined;
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
        objectUrl = undefined;
      }
    }
  };

  const handleUpload = async () => {
    const editor = ctx.editor;
    if (!editor || editor.blockBlobEdit()) {
      return;
    }

    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = 'image/*';
    picker.multiple = true;

    picker.addEventListener(
      'change',
      () => {
        pickerOpened = false;

        const files = picker.files ? [...picker.files].filter((file) => file.type.startsWith('image/')) : [];
        if (files.length === 0) {
          return;
        }

        const [firstFile, ...restFiles] = files;
        const uploadId = nanoid();
        editor.queueUpload(uploadId, firstFile);
        const queuedFile = editor.popUpload(uploadId) ?? firstFile;
        processedUploadId = uploadId;
        localUploadId = uploadId;
        void processFile(queuedFile, uploadId);

        if (restFiles.length > 0) {
          editor.insertImagesFromFiles(restFiles);
        }
      },
      { once: true },
    );

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

    const boundsWidth = element.bounds.width;
    if (boundsWidth <= 0) return;

    const dx = (event.clientX - initialResizeData.x) * (initialResizeData.reverse ? -1 : 1);
    const newWidth = clampWidth(initialResizeData.width + dx * 2, boundsWidth);
    proportion = Math.round((newWidth / boundsWidth) * 100);
  };

  const handleResizeEnd = (event: PointerEvent) => {
    const editor = ctx.editor;
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture(event.pointerId);
    isResizing = false;

    if (!editor || !imageData) return;
    editor.enqueue({
      type: 'node',
      op: {
        type: 'set_image_proportion',
        id: element.node_id,
        proportion: clampProportion(proportion),
      },
    });
    editor.focus();
  };

  $effect(() => {
    if (!imageData || isResizing) return;
    proportion = imageData.proportion ?? 100;
  });

  $effect(() => {
    const editor = ctx.editor;
    const image = imageAssetQuery.data?.image;
    if (!editor || !imageData?.id || !image || image.id !== imageData.id) return;

    editor.imageAssets.set(image.id, {
      id: image.id,
      url: image.url,
      originalUrl: image.originalUrl,
      width: image.width,
      height: image.height,
      placeholder: image.placeholder,
    });
  });

  $effect(() => {
    const editor = ctx.editor;
    const boundsWidth = element.bounds.width;
    if (!editor || !imageData || isResizing || boundsWidth <= 0 || originalWidth <= 0) return;

    const currentWidth = Math.min(originalWidth, boundsWidth * (proportion / 100));
    const clampedWidth = clampWidth(currentWidth, boundsWidth);

    if (currentWidth !== clampedWidth) {
      const newProportion = Math.round((clampedWidth / boundsWidth) * 100);
      proportion = newProportion;
      editor.enqueue({
        type: 'node',
        op: {
          type: 'set_image_proportion',
          id: element.node_id,
          proportion: clampProportion(newProportion),
        },
      });
    }
  });

  $effect(() => {
    if (!ctx.editor || !imageData || imageData.id || processedUploadId || !imageData.upload_id) return;

    const file = ctx.editor.popUpload(imageData.upload_id);
    if (file) {
      processedUploadId = imageData.upload_id;
      void processFile(file, imageData.upload_id);
    } else {
      console.warn('Upload file not found for uploadId:', imageData.upload_id);
    }
  });

  $effect(() => {
    if (!hasImage) {
      enlarged = false;
    }
  });

  $effect(() => {
    const editor = ctx.editor;
    const node = containerEl;
    if (!editor || !node) return;

    const observer = new ResizeObserver((entries) => {
      const height = entries[0]?.contentRect.height ?? 0;
      if (height <= 0 || height === reportedHeight) return;

      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }

      debounceTimer = setTimeout(() => {
        reportedHeight = height;
        editor.setExternalElementHeight(element.node_id, height);
        debounceTimer = null;
      }, 100);
    });

    observer.observe(node);
    return () => {
      observer.disconnect();
      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }
    };
  });

  $effect(() => {
    return () => {
      if (localUploadId) {
        ctx.editor?.clearImageUpload(localUploadId);
      }
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
        objectUrl = undefined;
      }
    };
  });
</script>

<div
  style:left={`${element.bounds.x}px`}
  style:top={`${element.bounds.y}px`}
  style:width={`${element.bounds.width}px`}
  class={css({
    position: 'absolute',
    userSelect: 'none',
    display: 'flex',
    justifyContent: 'center',
    visibility: reportedHeight === undefined ? 'hidden' : 'visible',
  })}
  data-external-element
  data-node-id={element.node_id}
>
  <div
    bind:this={containerEl}
    style:width={hasImage ? `${liveWidth}px` : '100%'}
    style:height={hasImage ? `${liveHeight}px` : undefined}
    class={cx('group', css({ position: 'relative', minHeight: '48px', margin: '[0 auto]' }))}
  >
    {#if hasImage && imageSrc}
      <Img
        style={css.raw({ width: 'full', borderRadius: '4px' })}
        alt="본문 이미지"
        aria-label="이미지 확대 보기"
        onclick={() => {
          enlarged = true;
        }}
        onkeydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            enlarged = true;
          }
        }}
        onpointerdown={(event) => event.stopPropagation()}
        placeholder={asset?.placeholder}
        progressive
        ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
        role="button"
        size="full"
        tabindex={0}
        url={imageSrc}
      />

      {#if isUploading}
        <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white/50' })}>
          <RingSpinner style={css.raw({ size: '24px', color: 'text.disabled' })} />
        </div>
      {/if}

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
        onclick={deleteNode}
        onpointerdown={(event) => event.stopPropagation()}
        type="button"
      >
        <Icon icon={Trash2Icon} size={16} />
      </button>

      {#each [true, false] as reverse (reverse)}
        <div
          class={flex({
            position: 'absolute',
            top: '0',
            bottom: '0',
            left: reverse ? '10px' : undefined,
            right: reverse ? undefined : '10px',
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
            onpointerdown={(event) => handleResizeStart(event, reverse)}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
          ></button>
        </div>
      {/each}
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
          {isResolvingAsset ? '이미지를 불러오는 중...' : '이미지'}
        </div>

        {#if isResolvingAsset}
          <div class={css({ marginRight: '14px' })}>
            <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
          </div>
        {/if}

        <div onpointerdown={stopPointerPropagation} role="presentation">
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

            <MenuItem onclick={deleteNode} variant="danger">
              <Icon icon={Trash2Icon} size={12} />
              <span>삭제</span>
            </MenuItem>
          </Menu>
        </div>
      </div>
    {/if}
  </div>

  {#if element.is_selected}
    <div
      style:background-color={selectionColor}
      style:opacity={selectionOpacity}
      class={css({
        position: 'absolute',
        inset: '0',
        borderRadius: '4px',
        pointerEvents: 'none',
      })}
    ></div>
  {/if}
</div>

{#if pickerOpened && !hasImage && !isResolvingAsset}
  <button
    class={flex({
      alignItems: 'center',
      gap: '6px',
      borderWidth: '1px',
      borderRadius: '8px',
      paddingX: '12px',
      paddingY: '6px',
      fontSize: '13px',
      color: 'text.muted',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      transition: 'common',
      zIndex: 'editor',
      _hover: { backgroundColor: 'interactive.hover' },
    })}
    onclick={handleUpload}
    onpointerdown={(event) => event.stopPropagation()}
    type="button"
    use:floating
  >
    <Icon icon={ImageIcon} size={14} />
    이미지 선택
  </button>
{/if}

{#if enlarged && hasImage && imageSrc && containerEl}
  <ExternalImageEnlarge
    onclose={() => (enlarged = false)}
    placeholder={asset?.placeholder}
    ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
    referenceEl={containerEl}
    url={imageSrc}
  />
{/if}
