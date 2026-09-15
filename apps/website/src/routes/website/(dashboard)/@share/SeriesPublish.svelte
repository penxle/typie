<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, Popover, Switch } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import ImageIcon from '~icons/lucide/image';
  import LinkIcon from '~icons/lucide/link';
  import SendIcon from '~icons/lucide/send';
  import StarIcon from '~icons/lucide/star';
  import TextIcon from '~icons/lucide/text';
  import XIcon from '~icons/lucide/x';
  import { LoadableImg } from '$lib/components';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import PropertyRow from './PropertyRow.svelte';
  import { linkFieldButtonStyle, linkFieldInputStyle, linkFieldStyle, propertyTextStyle, propertyTriggerStyle } from './publish-styles';
  import ShareHeader from './ShareHeader.svelte';
  import type { DashboardLayout_Share_SeriesPublish_folder$key } from '$mearie';

  type Props = {
    folder$key: DashboardLayout_Share_SeriesPublish_folder$key;
    onback: () => void;
    onclose: () => void;
  };

  let { folder$key, onback, onclose }: Props = $props();

  const folder = createFragment(
    graphql(`
      fragment DashboardLayout_Share_SeriesPublish_folder on Folder {
        id
        name
        description
        pinned
        seriesUrl

        thumbnail {
          id
        }

        entity {
          id
          visibility
        }
      }
    `),
    () => folder$key,
  );

  const [updateFoldersOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_SeriesPublish_UpdateFoldersOption_Mutation($input: UpdateFoldersOptionInput!) {
        updateFoldersOption(input: $input) {
          id
          description
          pinned
          seriesUrl

          thumbnail {
            id
          }

          entity {
            id
            visibility
          }
        }
      }
    `),
  );

  const existing = $derived(folder.data.entity.visibility === EntityVisibility.PUBLIC);

  let description = $state('');
  let thumbnailId = $state<string | null>(null);
  let thumbnailUploading = $state(false);
  let featured = $state(false);
  let descriptionOpen = $state(false);
  let descriptionEl = $state<HTMLTextAreaElement>();

  $effect(() => {
    const next = {
      description: folder.data.description ?? '',
      thumbnailId: folder.data.thumbnail?.id ?? null,
      featured: folder.data.pinned,
    };
    untrack(() => {
      description = next.description;
      thumbnailId = next.thumbnailId;
      featured = next.featured;
    });
  });

  const modified = $derived(
    description.trim() !== (folder.data.description ?? '') ||
      thumbnailId !== (folder.data.thumbnail?.id ?? null) ||
      featured !== folder.data.pinned,
  );

  let running = $state<'create' | 'save' | 'delete' | null>(null);

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    return () => {
      if (copyTimer) clearTimeout(copyTimer);
    };
  });

  const copyUrl = async () => {
    if (!folder.data.seriesUrl) return;
    await navigator.clipboard.writeText(folder.data.seriesUrl);
    if (copyTimer) clearTimeout(copyTimer);
    copied = true;
    copyTimer = setTimeout(() => (copied = false), 2000);
  };

  const selectUrl = (event: Event) => {
    (event.currentTarget as HTMLInputElement).select();
  };

  const submit = async (kind: 'create' | 'save') => {
    if (running) return;
    if (!SubscribeModal.gate('share_folder')) return;

    running = kind;
    try {
      await updateFoldersOption({
        input: {
          folderIds: [folder.data.id],
          ...(kind === 'create' && { visibility: EntityVisibility.PUBLIC }),
          description: description.trim() || null,
          thumbnailId,
          pinned: featured,
        },
      });
      mixpanel.track('update_folder_option', { series: kind });
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    } finally {
      running = null;
    }
  };

  const confirmDelete = () => {
    if (running) return;

    Dialog.confirm({
      title: '시리즈를 삭제하시겠어요?',
      message: '시리즈에 속한 글은 시리즈 없음이 되고, 글은 그대로 발행돼요.',
      action: 'danger',
      actionLabel: '시리즈 삭제',
      actionHandler: async () => {
        if (!SubscribeModal.gate('share_folder')) return;

        running = 'delete';
        try {
          await updateFoldersOption({ input: { folderIds: [folder.data.id], visibility: EntityVisibility.PRIVATE } });
          mixpanel.track('update_folder_option', { series: 'delete' });
          onback();
        } catch (err) {
          Toast.error(publicationErrorMessage(publicationErrorCode(err)));
        } finally {
          running = null;
        }
      },
    });
  };

  let scrollerEl = $state<HTMLElement>();
  let topSentinelEl = $state<HTMLElement>();
  let bottomSentinelEl = $state<HTMLElement>();
  let scrolledToTop = $state(true);
  let scrolledToBottom = $state(true);

  $effect(() => {
    const root = scrollerEl;
    const top = topSentinelEl;
    const bottom = bottomSentinelEl;
    if (!root || !top || !bottom) return;

    const observer = new IntersectionObserver(
      (records) => {
        for (const record of records) {
          if (record.target === top) scrolledToTop = record.isIntersecting;
          else scrolledToBottom = record.isIntersecting;
        }
      },
      { root },
    );

    observer.observe(top);
    observer.observe(bottom);

    return () => observer.disconnect();
  });

  $effect(() => {
    if (!descriptionOpen) return;

    const el = descriptionEl;
    if (!el) return;

    const frame = requestAnimationFrame(() => el.focus());
    return () => cancelAnimationFrame(frame);
  });

  const handleThumbnailUpload = () => {
    const input = window.document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      if (!file) return;

      thumbnailUploading = true;
      try {
        const image = await uploadBlobAsImage(file);
        thumbnailId = image.id;
      } finally {
        thumbnailUploading = false;
      }
    });

    input.click();
  };
</script>

<div class={css({ display: 'grid', gridTemplateColumns: '[360px minmax(0, 1fr)]', height: 'full', minHeight: '0' })}>
  <aside
    class={flex({
      flexDirection: 'column',
      gap: '12px',
      minHeight: '0',
      paddingX: '24px',
      paddingTop: '20px',
      borderRightWidth: '1px',
      borderColor: 'border.hairline',
      borderTopLeftRadius: '8px',
      borderBottomLeftRadius: '8px',
      backgroundColor: 'surface.canvas',
    })}
    aria-label="미리보기"
  ></aside>

  <section class={flex({ flexDirection: 'column', minHeight: '0' })}>
    <ShareHeader
      back={existing ? null : onback}
      bordered={!scrolledToTop}
      {onclose}
      subtitle={folder.data.name}
      title="스페이스에 시리즈로 발행"
    />

    <div
      bind:this={scrollerEl}
      class={css({ flex: '1', minHeight: '0', overflowY: 'auto', paddingTop: '2px', paddingX: '24px', paddingBottom: '24px' })}
    >
      <div bind:this={topSentinelEl} class={css({ height: '1px' })} aria-hidden="true"></div>

      <p class={css({ marginBottom: '16px', fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>
        이 폴더에 속한, 개별 발행된 글들을 순서 있는 묶음으로 모아 스페이스에 표시해요. 독자는 시리즈 페이지에서 폴더 내에 있는 글을 이전
        글·다음 글로 이어 읽을 수 있어요.
      </p>

      <div class={flex({ flexDirection: 'column', gap: '12px' })}>
        {#if existing}
          <div
            class={flex({
              position: 'relative',
              flexDirection: 'column',
              gap: '1px',
              marginX: '-8px',
              paddingBottom: '10px',
              _after: {
                content: '""',
                position: 'absolute',
                left: '8px',
                right: '8px',
                bottom: '0',
                height: '1px',
                backgroundColor: 'border.hairline',
              },
            })}
          >
            <PropertyRow icon={SendIcon} label="발행 상태">
              <span
                class={flex({ alignItems: 'center', gap: '8px', minHeight: '30px', paddingX: '8px', marginLeft: '-8px', fontSize: '13px' })}
              >
                <span class={css({ flexShrink: '0', size: '8px', borderRadius: 'full', backgroundColor: 'success.default' })}></span>
                <span class={css(propertyTextStyle)}>발행됨</span>
              </span>
            </PropertyRow>

            {#if folder.data.seriesUrl}
              <PropertyRow icon={LinkIcon} label="주소">
                <span class={css(linkFieldStyle)}>
                  <input
                    class={css(linkFieldInputStyle)}
                    aria-label="시리즈 주소"
                    autocomplete="off"
                    onclick={selectUrl}
                    onfocus={selectUrl}
                    readonly
                    spellcheck="false"
                    value={folder.data.seriesUrl}
                  />

                  <button
                    class={center(linkFieldButtonStyle)}
                    aria-label="링크 복사"
                    onclick={copyUrl}
                    type="button"
                    use:tooltip={{ message: copied ? '복사되었어요' : '링크 복사', placement: 'top', keepOnClick: true }}
                  >
                    <Icon
                      style={copied ? css.raw({ color: 'success.default' }) : undefined}
                      icon={copied ? CheckIcon : CopyIcon}
                      size={14}
                    />
                  </button>

                  <a
                    class={center(linkFieldButtonStyle)}
                    aria-label="시리즈 보기"
                    href={folder.data.seriesUrl}
                    rel="noopener noreferrer"
                    target="_blank"
                    use:tooltip={{ message: '시리즈 보기', placement: 'top' }}
                  >
                    <Icon icon={ExternalLinkIcon} size={14} />
                  </a>
                </span>
              </PropertyRow>
            {/if}
          </div>
        {/if}

        <PropertyRow icon={TextIcon} label="설명" top>
          <Popover
            style={css.raw(propertyTriggerStyle, { alignItems: 'flex-start', whiteSpace: 'normal' })}
            contentStyle={css.raw({
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              width: '380px',
              paddingX: '10px',
              paddingY: '10px',
            })}
            offset={4}
            placement="bottom-start"
            bind:open={descriptionOpen}
          >
            {#snippet trigger()}
              {#if description.trim().length > 0}
                <span class={css({ lineClamp: '2', lineHeight: '[1.5]', textAlign: 'left' })}>{description.trim()}</span>
              {:else}
                <span class={css({ lineClamp: '2', lineHeight: '[1.5]', textAlign: 'left', color: 'text.hint' })}>설명 쓰기</span>
              {/if}
            {/snippet}

            <textarea
              bind:this={descriptionEl}
              class={css({
                width: 'full',
                minHeight: '96px',
                padding: '10px',
                borderWidth: '1px',
                borderRadius: '6px',
                fontSize: '13px',
                lineHeight: '[1.6]',
                color: 'text.default',
                resize: 'none',
                _placeholder: { color: 'text.hint/60' },
                _focusWithin: { borderColor: 'accent.default' },
              })}
              aria-label="설명"
              placeholder="설명 쓰기"
              bind:value={description}></textarea>

            <div class={flex({ alignItems: 'center', justifyContent: 'flex-end', gap: '12px' })}>
              <span class={css({ fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>{description.length}자</span>
            </div>
          </Popover>
        </PropertyRow>

        <PropertyRow icon={ImageIcon} label="썸네일">
          {#if thumbnailId}
            <div class={css({ position: 'relative' })}>
              <button
                class={css({
                  position: 'relative',
                  display: 'block',
                  width: '36px',
                  aspectRatio: '[2 / 3]',
                  overflow: 'hidden',
                  borderRadius: '6px',
                  backgroundColor: 'surface.canvas',
                  _hover: { '& .thumbnail-overlay': { opacity: '100' } },
                })}
                aria-label="썸네일 변경"
                disabled={running !== null}
                onclick={handleThumbnailUpload}
                type="button"
              >
                <LoadableImg
                  id={thumbnailId}
                  style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                  alt="썸네일"
                  size={128}
                />

                <span
                  class={cx(
                    'thumbnail-overlay',
                    center({
                      position: 'absolute',
                      inset: '0',
                      fontSize: '11px',
                      fontWeight: 'semibold',
                      color: '[#fff]',
                      backgroundColor: '[rgba(9, 9, 12, 0.45)]',
                      opacity: '0',
                      transition: 'common',
                    }),
                  )}
                >
                  변경
                </span>
              </button>

              <button
                class={center({
                  position: 'absolute',
                  top: '-6px',
                  right: '-6px',
                  size: '18px',
                  borderWidth: '1px',
                  borderRadius: 'full',
                  color: 'text.muted',
                  backgroundColor: 'surface.default',
                  boxShadow: 'sm',
                  _hover: { color: 'danger.default' },
                })}
                aria-label="썸네일 삭제"
                onclick={() => (thumbnailId = null)}
                type="button"
                use:tooltip={{ message: '삭제', placement: 'top' }}
              >
                <Icon icon={XIcon} size={10} />
              </button>
            </div>
          {:else}
            <button
              class={css(propertyTriggerStyle, {
                color: 'text.muted',
                _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
              })}
              disabled={running !== null || thumbnailUploading}
              onclick={handleThumbnailUpload}
              type="button"
            >
              <Icon icon={ImageIcon} size={14} />
              <span>{thumbnailUploading ? '올리는 중' : '썸네일 올리기'}</span>
            </button>
          {/if}
        </PropertyRow>

        <PropertyRow hint="스페이스의 대표 시리즈로 노출해요. 여러개를 노출할 수 있어요." icon={StarIcon} label="고정">
          <Switch bind:checked={featured} />
        </PropertyRow>
      </div>

      <div bind:this={bottomSentinelEl} class={css({ height: '1px' })} aria-hidden="true"></div>
    </div>

    <div
      class={flex({
        alignItems: 'center',
        gap: '10px',
        paddingTop: '14px',
        paddingX: '24px',
        paddingBottom: '16px',
        borderTopWidth: '1px',
        borderColor: scrolledToBottom ? 'transparent' : 'border.hairline',
        transition: 'common',
      })}
    >
      {#if existing}
        <Button
          style={css.raw({ color: 'danger.default' })}
          disabled={running !== null}
          loading={running === 'delete'}
          onclick={confirmDelete}
          size="md"
          variant="ghost"
        >
          시리즈 삭제
        </Button>
      {/if}

      <div class={flex({ alignItems: 'center', gap: '8px', marginLeft: 'auto' })}>
        {#if existing && !modified}
          <span class={css({ fontSize: '12px', color: 'text.hint' })}>바뀐 내용이 없어요</span>
        {/if}

        {#if existing}
          <Button
            disabled={!modified || running !== null}
            loading={running === 'save'}
            onclick={() => submit('save')}
            size="md"
            variant={modified ? 'primary' : 'secondary'}
          >
            시리즈 저장
          </Button>
        {:else}
          <Button disabled={running !== null} loading={running === 'create'} onclick={() => submit('create')} size="md" variant="primary">
            시리즈 생성
          </Button>
        {/if}
      </div>
    </div>
  </section>
</div>
