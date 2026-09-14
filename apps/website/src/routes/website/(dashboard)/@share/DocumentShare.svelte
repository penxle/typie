<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import XIcon from '~icons/lucide/x';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import {
    composeScheduledAt,
    defaultScheduleParts,
    isScheduleInFuture,
    modifiedPublishFields,
    publicationErrorMessage,
    resolvePublishView,
    scheduleParts,
  } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import NewSpaceForm from '../@site-settings/NewSpaceForm.svelte';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import PublishPreview from './PublishPreview.svelte';
  import PublishProperties from './PublishProperties.svelte';
  import PublishStatusRows from './PublishStatusRows.svelte';
  import ReadingSettings from './ReadingSettings.svelte';
  import VisibilityStep from './VisibilityStep.svelte';
  import type { PublishAction, PublishMode, ScheduleParts } from '$lib/publication/publish-form';
  import type { DashboardLayout_Share_DocumentShare_document$key } from '$mearie';

  type Props = {
    document$key: DashboardLayout_Share_DocumentShare_document$key;
    step: 'visibility' | 'publish';
    onclose: () => void;
  };

  let { document$key, step = $bindable(), onclose }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DashboardLayout_Share_DocumentShare_document on Document {
        id
        title
        subtitle
        excerpt
        password

        thumbnail {
          id
        }

        entity {
          id
          url
          visibility

          site {
            id

            logo {
              id
              ...Img_image
            }

            spaces {
              id
              name
              slug
              url

              logo {
                id
                ...Img_image
              }

              collections {
                id
                name
              }
            }

            ...DashboardLayout_Share_PublishProperties_site
          }
        }

        publication {
          id
          state
          scheduledAt
          publishedAt
          hasUnpublishedChanges
          tags
          excerpt
          url

          thumbnail {
            id
          }

          collection {
            id
            name
          }

          space {
            id
          }
        }

        ...DashboardLayout_Share_VisibilityStep_document
        ...DashboardLayout_Share_ReadingSettings_document
      }
    `),
    () => document$key,
  );

  const [publishDocument, publishResult] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_DocumentShare_PublishDocument_Mutation($input: PublishDocumentInput!) {
        publishDocument(input: $input) {
          id
          state
          scheduledAt
          publishedAt
          hasUnpublishedChanges
          tags
          excerpt
          url

          thumbnail {
            id
          }

          collection {
            id
            name
          }

          space {
            id
          }

          document {
            id

            publication {
              id
            }

            entity {
              id
              visibility
            }
          }
        }
      }
    `),
  );

  const [updatePublication, updateResult] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_DocumentShare_UpdatePublication_Mutation($input: UpdatePublicationInput!) {
        updatePublication(input: $input) {
          id
          state
          scheduledAt
          publishedAt
          hasUnpublishedChanges
          tags
          excerpt
          url

          thumbnail {
            id
          }

          collection {
            id
            name
          }

          space {
            id
          }

          document {
            id

            publication {
              id
            }

            entity {
              id
              visibility
            }
          }
        }
      }
    `),
  );

  const [unpublishDocument, unpublishResult] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_DocumentShare_UnpublishDocument_Mutation($input: UnpublishDocumentInput!) {
        unpublishDocument(input: $input) {
          id
          state
          scheduledAt
          publishedAt
          hasUnpublishedChanges
          tags
          excerpt
          url

          thumbnail {
            id
          }

          collection {
            id
            name
          }

          space {
            id
          }

          document {
            id

            publication {
              id
            }

            entity {
              id
              visibility
            }
          }
        }
      }
    `),
  );

  const [cancelScheduledPublication, cancelResult] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_DocumentShare_CancelScheduledPublication_Mutation($input: CancelScheduledPublicationInput!) {
        cancelScheduledPublication(input: $input) {
          id
          state
          scheduledAt
          publishedAt
          hasUnpublishedChanges
          tags
          excerpt
          url

          thumbnail {
            id
          }

          collection {
            id
            name
          }

          space {
            id
          }

          document {
            id

            publication {
              id
            }

            entity {
              id
              visibility
            }
          }
        }
      }
    `),
  );

  const publication = $derived(document.data.publication);
  const spaces = $derived(document.data.entity.site.spaces);
  const initial = document.data.publication;

  let spaceId = $state<string | null>(initial?.space.id ?? null);
  let collectionId = $state<string | null>(initial?.collection?.id ?? null);
  let tags = $state<string[]>([...(initial?.tags ?? [])]);
  let excerpt = $state(initial?.excerpt ?? '');
  let thumbnailId = $state<string | null>(initial ? (initial.thumbnail?.id ?? null) : (document.data.thumbnail?.id ?? null));
  let mode = $state<PublishMode>(initial?.state === 'SCHEDULED' ? 'schedule' : 'now');
  let schedule = $state<ScheduleParts>(initial?.scheduledAt ? scheduleParts(initial.scheduledAt) : defaultScheduleParts(dayjs()));

  const currentSpaceId = $derived(spaceId ?? publication?.space.id ?? spaces[0]?.id ?? null);
  const space = $derived(spaces.find((s) => s.id === currentSpaceId) ?? spaces[0] ?? null);
  const collection = $derived(space?.collections.find((c) => c.id === collectionId) ?? null);

  const draft = $derived({
    spaceId: currentSpaceId,
    collectionId,
    tags,
    excerpt,
    thumbnailId,
    mode,
    schedule,
  });

  const summary = $derived(
    publication
      ? {
          state: publication.state,
          scheduledAt: publication.scheduledAt ?? null,
          publishedAt: publication.publishedAt ?? null,
          hasUnpublishedChanges: publication.hasUnpublishedChanges,
          spaceId: publication.space.id,
          collectionId: publication.collection?.id ?? null,
          tags: publication.tags,
          excerpt: publication.excerpt ?? null,
          thumbnailId: publication.thumbnail?.id ?? null,
        }
      : null,
  );

  const view = $derived(resolvePublishView(summary, draft));
  const modified = $derived(modifiedPublishFields(summary, draft));

  const busy = $derived(publishResult.loading || updateResult.loading || unpublishResult.loading || cancelResult.loading);

  let running = $state<PublishAction | null>(null);

  const invalidatePublicationLists = (targetSpaceId: string, collectionIds: (string | null)[]) => {
    cache.invalidate({ __typename: 'Space', id: targetSpaceId, $field: 'publications' });
    cache.invalidate({ __typename: 'Space', id: targetSpaceId, $field: 'pinnedPublications' });

    for (const id of new Set(collectionIds.filter((value): value is string => value !== null))) {
      cache.invalidate({ __typename: 'Collection', id, $field: 'publications' });
    }
  };

  const submit = async (action: PublishAction) => {
    if (busy) return;

    const meta = { tags, collectionId, excerpt: excerpt.trim() || null, thumbnailId };
    const previousCollectionId = publication?.collection?.id ?? null;

    running = action;

    try {
      if (action === 'publish' || action === 'schedule' || action === 'reschedule' || action === 'publishNow') {
        if (!currentSpaceId || !SubscribeModal.gate('publish_document')) return;

        const scheduling = action === 'schedule' || action === 'reschedule';
        if (scheduling && !isScheduleInFuture(schedule, dayjs())) {
          Toast.error(publicationErrorMessage('publication_scheduled_in_past'));
          return;
        }

        const targetSpaceId = currentSpaceId;

        await publishDocument({
          input: {
            documentId: document.data.id,
            spaceId: targetSpaceId,
            ...meta,
            scheduledAt: scheduling ? composeScheduledAt(schedule) : null,
          },
        });

        invalidatePublicationLists(targetSpaceId, [previousCollectionId, meta.collectionId]);

        if (action === 'reschedule') {
          Toast.success('예약에 반영됐어요.');
        } else {
          Toast.success(action === 'schedule' ? '발행이 예약됐어요.' : '발행됐어요.');
        }

        if (action === 'publishNow') {
          mode = 'now';
        }

        mixpanel.track('publish_document', {
          scheduled: scheduling,
          tags: tags.length,
          collection: collectionId !== null,
          from: action,
        });
      } else if (action === 'republish') {
        if (!publication || !SubscribeModal.gate('publish_document')) return;

        const targetSpaceId = publication.space.id;

        await updatePublication({ input: { publicationId: publication.id, ...meta } });
        invalidatePublicationLists(targetSpaceId, [previousCollectionId, meta.collectionId]);
        Toast.success('다시 발행됐어요.');
        mixpanel.track('update_publication', { tags: tags.length, collection: collectionId !== null });
      } else if (action === 'cancel') {
        if (!publication) return;

        const targetSpaceId = publication.space.id;

        await cancelScheduledPublication({ input: { publicationId: publication.id } });
        invalidatePublicationLists(targetSpaceId, [previousCollectionId]);
        mode = 'now';
        schedule = defaultScheduleParts(dayjs());
        Toast.success('예약이 취소됐어요.');
        mixpanel.track('cancel_scheduled_publication');
      }
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    } finally {
      running = null;
    }
  };

  const confirmUnpublish = () => {
    if (!publication || busy) return;

    const targetSpaceId = publication.space.id;
    const previousCollectionId = publication.collection?.id ?? null;

    Dialog.confirm({
      title: '발행을 취소하시겠어요?',
      message: '스페이스에서 글이 내려가고, 문서는 나만 볼 수 있게 돼요.',
      action: 'danger',
      actionLabel: '발행 취소',
      actionHandler: async () => {
        running = 'unpublish';

        try {
          await unpublishDocument({ input: { documentId: document.data.id } });
          invalidatePublicationLists(targetSpaceId, [previousCollectionId]);
          Toast.success('발행이 취소됐어요.');
          mixpanel.track('unpublish_document');
        } catch (err) {
          Toast.error(publicationErrorMessage(publicationErrorCode(err)));
        } finally {
          running = null;
        }
      },
    });
  };

  const confirmPublishNow = () => {
    if (busy) return;

    Dialog.confirm({
      title: '지금 발행하시겠어요?',
      message: '예약한 시각을 지우고 바로 올라가요.',
      actionLabel: '지금 발행',
      actionHandler: async () => {
        await submit('publishNow');
      },
    });
  };

  const run = (action: PublishAction) => {
    if (action === 'unpublish') {
      confirmUnpublish();
    } else if (action === 'publishNow') {
      confirmPublishNow();
    } else {
      void submit(action);
    }
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

  const previewDate = $derived(
    publication?.publishedAt ? dayjs(publication.publishedAt).toDate() : mode === 'schedule' ? schedule.date : new Date(),
  );
</script>

{#snippet header(title: string, back: boolean, bordered: boolean)}
  <div
    class={flex({
      position: 'relative',
      alignItems: 'center',
      gap: '8px',
      minWidth: '0',
      paddingTop: '22px',
      paddingRight: '60px',
      paddingBottom: '16px',
      paddingLeft: '24px',
      borderBottomWidth: '1px',
      borderColor: bordered ? 'border.hairline' : 'transparent',
      transition: 'common',
    })}
  >
    {#if back}
      <button
        class={center({
          flexShrink: '0',
          size: '28px',
          marginLeft: '-8px',
          borderRadius: '6px',
          color: 'text.muted',
          _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
        })}
        aria-label="뒤로"
        onclick={() => (step = 'visibility')}
        type="button"
        use:tooltip={{ message: '뒤로', placement: 'bottom' }}
      >
        <Icon icon={ChevronLeftIcon} size={18} />
      </button>
    {/if}

    <h2 class={css({ flexShrink: '0', fontSize: '17px', fontWeight: 'semibold', whiteSpace: 'nowrap' })}>{title}</h2>
    <span
      class={css({
        minWidth: '0',
        overflow: 'hidden',
        fontSize: '13px',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
        color: 'text.muted',
      })}
    >
      {document.data.title}
    </span>

    <button
      class={center({
        position: 'absolute',
        top: '14px',
        right: '14px',
        size: '28px',
        borderRadius: '6px',
        color: 'text.muted',
        transition: 'common',
        _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
      })}
      aria-label="닫기"
      onclick={onclose}
      type="button"
    >
      <Icon icon={XIcon} size={18} />
    </button>
  </div>
{/snippet}

{#if step === 'visibility'}
  {@render header('공유 및 발행', false, false)}
  <VisibilityStep document$key={document.data} onPublishStep={() => (step = 'publish')} />
{:else}
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
    >
      {#if space}
        <PublishPreview
          collectionName={collection?.name ?? null}
          date={previewDate}
          excerpt={excerpt.trim() || document.data.excerpt}
          locked={document.data.password !== null}
          spaceLogo$key={space.logo ?? document.data.entity.site.logo}
          spaceName={space.name}
          spaceSlug={space.slug}
          spaceUrl={space.url}
          subtitle={document.data.subtitle ?? null}
          {tags}
          {thumbnailId}
          title={document.data.title}
        />
      {/if}
    </aside>

    <section class={flex({ flexDirection: 'column', minHeight: '0' })}>
      {@render header('스페이스에 발행', view.canGoBack, !scrolledToTop)}

      <div
        bind:this={scrollerEl}
        class={css({ flex: '1', minHeight: '0', overflowY: 'auto', paddingTop: '2px', paddingX: '24px', paddingBottom: '24px' })}
      >
        <div bind:this={topSentinelEl} class={css({ height: '1px' })} aria-hidden="true"></div>

        {#if spaces.length === 0}
          <div class={flex({ flexDirection: 'column', gap: '12px' })}>
            <div class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.hint' })}>새 스페이스</div>
            <NewSpaceForm oncreated={(created) => (spaceId = created.id)} siteId={document.data.entity.site.id} />
          </div>
        {:else}
          <div class={flex({ flexDirection: 'column', gap: '12px' })}>
            {#if publication && publication.state !== 'UNPUBLISHED'}
              <PublishStatusRows
                linkShared={document.data.entity.visibility === EntityVisibility.UNLISTED}
                modified={publication.hasUnpublishedChanges}
                publicationState={publication.state === 'SCHEDULED' ? 'SCHEDULED' : 'PUBLISHED'}
                publishedAt={publication.publishedAt ?? null}
                scheduledAt={publication.scheduledAt ?? null}
                url={publication.url}
              />
            {/if}

            <PublishProperties
              autoExcerpt={document.data.excerpt}
              disabled={busy}
              {modified}
              scheduleEditable={view.scheduleEditable}
              site$key={document.data.entity.site}
              spaceSelectable={view.spaceSelectable}
              bind:spaceId
              bind:collectionId
              bind:tags
              bind:excerpt
              bind:thumbnailId
              bind:mode
              bind:schedule
            />

            <div
              class={css({
                marginTop: '10px',
                paddingTop: '14px',
                borderTopWidth: '1px',
                borderColor: 'border.hairline',
                fontSize: '12px',
                fontWeight: 'semibold',
                color: 'text.hint',
              })}
            >
              읽기 설정
            </div>

            <ReadingSettings document$key={document.data} />
          </div>
        {/if}

        <div bind:this={bottomSentinelEl} class={css({ height: '1px' })} aria-hidden="true"></div>
      </div>

      {#if spaces.length > 0}
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
          {#if view.destructive}
            {@const destructive = view.destructive}
            <Button
              style={css.raw({ color: destructive.action === 'unpublish' ? 'danger.default' : 'text.muted' })}
              disabled={busy}
              loading={running === destructive.action}
              onclick={() => run(destructive.action)}
              size="md"
              variant="ghost"
            >
              {destructive.label}
            </Button>
          {/if}

          <div class={flex({ alignItems: 'center', gap: '8px', marginLeft: 'auto' })}>
            {#if view.kind === 'published' && view.primary && !view.primary.enabled}
              <span class={css({ fontSize: '12px', color: 'text.hint' })}>바뀐 내용이 없어요</span>
            {/if}

            {#if view.secondary}
              {@const secondary = view.secondary}
              <Button
                disabled={busy}
                loading={running === secondary.action}
                onclick={() => run(secondary.action)}
                size="md"
                variant="secondary"
              >
                {secondary.label}
              </Button>
            {/if}

            {#if view.primary}
              {@const primary = view.primary}
              <Button
                disabled={!primary.enabled || busy}
                loading={running === primary.action}
                onclick={() => run(primary.action)}
                size="md"
                variant={primary.enabled ? 'primary' : 'secondary'}
              >
                {primary.label}
              </Button>
            {/if}
          </div>
        </div>
      {/if}
    </section>
  </div>
{/if}
