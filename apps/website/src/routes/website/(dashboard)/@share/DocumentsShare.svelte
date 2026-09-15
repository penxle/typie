<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode, publicationErrorDocumentId } from '$lib/publication/error';
  import {
    bulkPublishSuccessMessage,
    bulkScheduledAtInput,
    bulkTagDraft,
    defaultScheduleParts,
    initialBulkSchedule,
    isScheduleInFuture,
    partitionTags,
    publicationErrorMessage,
    resolveBulkPublishView,
    sharedValue,
  } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import PublishProperties from './PublishProperties.svelte';
  import PublishStatusSummary from './PublishStatusSummary.svelte';
  import ReadingSettings from './ReadingSettings.svelte';
  import SelectedDocuments from './SelectedDocuments.svelte';
  import ShareHeader from './ShareHeader.svelte';
  import VisibilityStep from './VisibilityStep.svelte';
  import type {
    BulkDocumentSummary,
    BulkPublishDraft,
    BulkPublishView,
    BulkScheduleInitial,
    BulkScheduleMode,
    ScheduleParts,
  } from '$lib/publication/publish-form';
  import type { DashboardLayout_Share_DocumentsShare_document$key } from '$mearie';

  type Props = {
    documents$key: DashboardLayout_Share_DocumentsShare_document$key[];
    step: 'visibility' | 'publish';
    onclose: () => void;
  };

  let { documents$key, step = $bindable(), onclose }: Props = $props();

  const documents = createFragment(
    graphql(`
      fragment DashboardLayout_Share_DocumentsShare_document on Document {
        id
        title

        entity {
          id
          url
          visibility

          site {
            id
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
        }

        ...DashboardLayout_Share_VisibilityStep_document
        ...DashboardLayout_Share_ReadingSettings_document
      }
    `),
    () => documents$key,
  );

  const [publishDocuments, publishResult] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_DocumentsShare_PublishDocuments_Mutation($input: PublishDocumentsInput!) {
        publishDocuments(input: $input) {
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

  const [unpublishDocuments, unpublishResult] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_DocumentsShare_UnpublishDocuments_Mutation($input: UnpublishDocumentsInput!) {
        unpublishDocuments(input: $input) {
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

  const summaries = $derived<BulkDocumentSummary[]>(
    documents.data.map((document) => ({
      id: document.id,
      publication: document.publication
        ? {
            state: document.publication.state,
            scheduledAt: document.publication.scheduledAt ?? null,
            publishedAt: document.publication.publishedAt ?? null,
            hasUnpublishedChanges: document.publication.hasUnpublishedChanges,
            tags: document.publication.tags,
            excerpt: document.publication.excerpt ?? null,
            thumbnailId: document.publication.thumbnail?.id ?? null,
          }
        : null,
    })),
  );

  const site = $derived(documents.data[0].entity.site);

  let initialSchedule = $state<BulkScheduleInitial>({ mode: 'now', schedule: defaultScheduleParts(dayjs()) });
  let tagBase = $state<ReturnType<typeof partitionTags>>({ common: [], partial: [] });
  let tags = $state<string[]>([]);
  let partial = $state<{ tag: string; count: number }[]>([]);
  let scheduleMode = $state<BulkScheduleMode>('now');
  let schedule = $state<ScheduleParts>(defaultScheduleParts(dayjs()));
  let excerpt = $state('');
  let thumbnailId = $state<string | null>(null);

  const resetDraft = () => {
    initialSchedule = initialBulkSchedule(summaries, dayjs());
    tagBase = partitionTags(documents.data.map((document) => document.publication?.tags ?? []));
    tags = [...tagBase.common];
    partial = [...tagBase.partial];
    scheduleMode = initialSchedule.mode;
    schedule = initialSchedule.schedule;
  };

  resetDraft();

  const tagDraft = $derived(
    bulkTagDraft(
      { common: tagBase.common, partial: tagBase.partial.map((item) => item.tag) },
      { tags, partial: partial.map((item) => item.tag) },
    ),
  );

  const draft = $derived<BulkPublishDraft>({
    addTags: tagDraft.addTags,
    removeTags: tagDraft.removeTags,
    scheduleMode,
    schedule,
  });

  const view = $derived(resolveBulkPublishView(summaries, draft, initialSchedule));

  const publishedUrls = $derived(
    documents.data
      .filter((document) => document.publication?.state === 'PUBLISHED')
      .map((document) => document.publication?.url ?? '')
      .filter(Boolean),
  );
  const modifiedScheduled = $derived(
    documents.data.filter((document) => document.publication?.state === 'SCHEDULED' && document.publication.hasUnpublishedChanges).length,
  );
  const modifiedPublished = $derived(
    documents.data.filter((document) => document.publication?.state === 'PUBLISHED' && document.publication.hasUnpublishedChanges).length,
  );
  const scheduledLinkShared = $derived.by(() => {
    const shared = sharedValue(
      documents.data.filter((document) => document.publication?.state === 'SCHEDULED').map((document) => document.entity.visibility),
    );
    return shared === undefined ? null : shared === EntityVisibility.UNLISTED;
  });

  const busy = $derived(publishResult.loading || unpublishResult.loading);

  let running = $state<'publish' | 'unpublish' | 'cancel' | null>(null);
  let failed = $state<{ documentId: string; message: string } | null>(null);

  const invalidateLists = () => {
    cache.invalidate({ __typename: 'SiteView', id: site.id, $field: 'recentPublications' });
    cache.invalidate({ __typename: 'SiteView', id: site.id, $field: 'entries' });
    cache.invalidate({ __typename: 'SiteView', id: site.id, $field: 'pinnedPublications' });
  };

  const fail = (err: unknown) => {
    const message = publicationErrorMessage(publicationErrorCode(err));
    const documentId = publicationErrorDocumentId(err);
    const title = documentId ? documents.data.find((document) => document.id === documentId)?.title : undefined;

    failed = documentId ? { documentId, message } : null;
    Toast.error(title ? `『${title}』: ${message}` : message);
  };

  const submit = async () => {
    if (busy || !view.primary.enabled) return;
    if (!SubscribeModal.gate('publish_document')) return;

    if (draft.scheduleMode === 'schedule' && view.modified.schedule && !isScheduleInFuture(schedule, dayjs())) {
      Toast.error(publicationErrorMessage('publication_scheduled_in_past'));
      return;
    }

    const snapshot: BulkPublishView = view;
    const mode = draft.scheduleMode;
    const changes = {
      addTags: tagDraft.addTags,
      removeTags: tagDraft.removeTags,
    };
    const scheduledAt = bulkScheduledAtInput(draft, snapshot.modified);

    running = 'publish';

    try {
      await publishDocuments({
        input: {
          documentIds: snapshot.targetIds,
          ...(changes.addTags.length > 0 && { addTags: changes.addTags }),
          ...(changes.removeTags.length > 0 && { removeTags: changes.removeTags }),
          ...(scheduledAt !== undefined && { scheduledAt }),
        },
      });

      invalidateLists();
      failed = null;
      await tick();
      resetDraft();
      Toast.success(bulkPublishSuccessMessage(snapshot, mode));
      mixpanel.track('publish_documents', {
        count: snapshot.targetIds.length,
        mixed: snapshot.mixed,
        scheduled: mode === 'schedule',
        tags: changes.addTags.length + changes.removeTags.length > 0,
      });
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  };

  const confirmPublishNow = () => {
    if (busy) return;

    Dialog.confirm({
      title: `글 ${view.counts.scheduled}개를 지금 발행하시겠어요?`,
      message: '예약한 시각을 지우고 바로 올라가요.',
      actionLabel: '지금 발행',
      actionHandler: async () => {
        await submit();
      },
    });
  };

  const runPrimary = () => {
    if (view.publishNowConfirm) {
      confirmPublishNow();
    } else {
      void submit();
    }
  };

  const runDestructive = (item: BulkPublishView['destructive'][number]) => {
    if (busy) return;

    const execute = async () => {
      running = item.action;

      try {
        await unpublishDocuments({ input: { documentIds: item.documentIds } });

        invalidateLists();
        failed = null;
        await tick();
        resetDraft();
        Toast.success(item.action === 'unpublish' ? '발행이 취소됐어요.' : '예약이 취소됐어요.');
        mixpanel.track(item.action === 'unpublish' ? 'unpublish_documents' : 'cancel_scheduled_publications', {
          count: item.documentIds.length,
        });
      } catch (err) {
        fail(err);
      } finally {
        running = null;
      }
    };

    if (item.action === 'unpublish') {
      Dialog.confirm({
        title: `글 ${item.documentIds.length}개의 발행을 취소하시겠어요?`,
        message: '스페이스에서 글이 내려가고, 문서는 나만 볼 수 있게 돼요.',
        action: 'danger',
        actionLabel: '발행 취소',
        actionHandler: execute,
      });
    } else {
      Dialog.confirm({
        title: `글 ${item.documentIds.length}개의 예약을 취소하시겠어요?`,
        message: '예약한 시각이 지워져요.',
        actionLabel: '예약 취소',
        actionHandler: execute,
      });
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

  const subtitle = $derived(`글 ${documents.data.length}개`);
</script>

{#if step === 'visibility'}
  <ShareHeader {onclose} {subtitle} title="공유 및 발행" />
  <VisibilityStep documents$key={documents.data} onPublishStep={() => (step = 'publish')} />
{:else}
  <div class={css({ display: 'grid', gridTemplateColumns: '[360px minmax(0, 1fr)]', height: 'full', minHeight: '0' })}>
    <aside
      class={flex({
        flexDirection: 'column',
        gap: '12px',
        minHeight: '0',
        paddingX: '24px',
        paddingTop: '20px',
        paddingBottom: '20px',
        borderRightWidth: '1px',
        borderColor: 'border.hairline',
        borderTopLeftRadius: '8px',
        borderBottomLeftRadius: '8px',
        backgroundColor: 'surface.canvas',
      })}
      aria-label="선택한 글"
    >
      <SelectedDocuments
        documents={documents.data.map((document) => ({
          id: document.id,
          title: document.title,
          state: document.publication?.state ?? 'UNPUBLISHED',
          modified: document.publication?.hasUnpublishedChanges ?? false,
        }))}
        {failed}
      />
    </aside>

    <section class={flex({ flexDirection: 'column', minHeight: '0' })}>
      <ShareHeader
        back={view.canGoBack ? () => (step = 'visibility') : null}
        bordered={!scrolledToTop}
        {onclose}
        {subtitle}
        title="스페이스에 발행"
      />

      <div
        bind:this={scrollerEl}
        class={css({ flex: '1', minHeight: '0', overflowY: 'auto', paddingTop: '2px', paddingX: '24px', paddingBottom: '24px' })}
      >
        <div bind:this={topSentinelEl} class={css({ height: '1px' })} aria-hidden="true"></div>

        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          {#if view.mixed}
            <PublishStatusSummary counts={view.counts} {modifiedPublished} {modifiedScheduled} {scheduledLinkShared} urls={publishedUrls} />
          {/if}

          <PublishProperties
            autoExcerpt=""
            disabled={busy}
            metaHint="미리보기 문구와 썸네일은 글마다 달라서 각 글에서 정해요."
            modified={view.modified}
            onremovepartialtag={(tag) => (partial = partial.filter((item) => item.tag !== tag))}
            partialTags={partial}
            scheduleEditable={view.scheduleEditable}
            showMeta={false}
            siteId={site.id}
            bind:tags
            bind:excerpt
            bind:thumbnailId
            bind:mode={scheduleMode}
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

          <ReadingSettings documents$key={documents.data} />
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
        {#each view.destructive as item (item.action)}
          <Button
            style={css.raw({ color: item.action === 'unpublish' ? 'danger.default' : 'text.muted' })}
            disabled={busy}
            loading={running === item.action}
            onclick={() => runDestructive(item)}
            size="md"
            variant="ghost"
          >
            {item.label}
          </Button>
        {/each}

        <div class={flex({ alignItems: 'center', gap: '8px', marginLeft: 'auto' })}>
          {#if view.mixed && !view.primary.enabled}
            <span class={css({ fontSize: '12px', color: 'text.hint' })}>바뀐 내용이 없어요</span>
          {/if}

          <Button
            disabled={!view.primary.enabled || busy}
            loading={running === 'publish'}
            onclick={runPrimary}
            size="md"
            variant={view.primary.enabled ? 'primary' : 'secondary'}
          >
            {view.primary.label}
          </Button>
        </div>
      </div>
    </section>
  </div>
{/if}
