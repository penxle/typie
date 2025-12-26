<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Select } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { EntityAvailability, EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_Share_Document_document } from '$graphql';

  type Props = {
    $documents: DashboardLayout_Share_Document_document[];
  };

  let { $documents: _documents }: Props = $props();

  const documents = fragment(
    _documents,
    graphql(`
      fragment DashboardLayout_Share_Document_document on Document {
        id
        title

        entity {
          id
          url
          slug
          visibility
          availability
        }
      }
    `),
  );

  let activeTab = $state<'publish' | 'share'>('publish');

  const isSingleDocument = $derived($documents.length === 1);
  const documentIds = $derived($documents.map((d) => d.id));

  const updateDocumentsOption = graphql(`
    mutation DashboardLayout_Share_Document_UpdateDocumentsOption_Mutation($input: UpdateDocumentsOptionInput!) {
      updateDocumentsOption(input: $input) {
        id

        entity {
          id
          visibility
          availability
        }
      }
    }
  `);

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  const visibilityIndeterminate = $derived(
    $documents.length > 1 && $documents.some((d) => d.entity.visibility !== $documents[0].entity.visibility),
  );
  const availabilityIndeterminate = $derived(
    $documents.length > 1 && $documents.some((d) => d.entity.availability !== $documents[0].entity.availability),
  );

  const form = createForm({
    schema: z.object({
      availability: z.nativeEnum(EntityAvailability),
      visibility: z.nativeEnum(EntityVisibility),
    }),
    submitOn: 'change',
    onSubmit: async (data) => {
      if ($documents.length === 0) return;

      const dirtyFields = form.getDirtyFields();
      const updateData: {
        documentIds: string[];
        availability?: EntityAvailability;
        visibility?: EntityVisibility;
      } = { documentIds };

      if ('availability' in dirtyFields) updateData.availability = data.availability;
      if ('visibility' in dirtyFields) updateData.visibility = data.visibility;

      if (Object.keys(updateData).length > 1) {
        await updateDocumentsOption(updateData);

        mixpanel.track('update_document_option', {
          ...updateData,
          count: $documents.length,
        });
      }
    },
    defaultValues: {
      availability: $documents[0].entity.availability,
      visibility: $documents[0].entity.visibility,
    },
  });

  $effect(() => {
    void form;
  });

  $effect(() => {
    return () => {
      if (timer) {
        clearTimeout(timer);
      }
    };
  });

  const handleCopyLink = () => {
    if ($documents.length === 0) return;

    const urls = $documents.map((d) => (activeTab === 'publish' ? d.entity.url : `${env.PUBLIC_WEBSITE_URL}/${d.entity.slug}`)).join('\n');
    navigator.clipboard.writeText(urls);
    mixpanel.track('copy_document_share_url', { tab: activeTab, count: $documents.length });

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1', fontWeight: 'semibold' })}>
      {isSingleDocument ? $documents[0].title : `${$documents.length}개의 문서`}
    </span>
    <span class={css({ flexShrink: '0' })}>공유 및 게시하기</span>
  </div>
  <button
    class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
    onclick={handleCopyLink}
    type="button"
    use:tooltip={{
      message:
        activeTab === 'publish'
          ? visibilityIndeterminate
            ? null
            : form.fields.visibility === EntityVisibility.PRIVATE
              ? '지금은 링크가 있어도 나만 볼 수 있어요'
              : '링크가 있는 누구나 문서를 볼 수 있어요'
          : availabilityIndeterminate
            ? null
            : form.fields.availability === EntityAvailability.PRIVATE
              ? '지금은 링크가 있어도 나만 편집할 수 있어요'
              : '링크가 있는 누구나 편집할 수 있어요',
      placement: 'top',
      keepOnClick: true,
    }}
  >
    {#if copied}
      <Icon style={css.raw({ color: 'text.link' })} icon={CheckIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.link' })}>복사되었어요</div>
    {:else}
      <Icon style={css.raw({ color: 'text.link' })} icon={LinkIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.link' })}>
        {activeTab === 'publish' ? '조회' : '편집'}
        {isSingleDocument ? '링크' : '링크 모두'} 복사
      </div>
    {/if}
  </button>
</div>

<HorizontalDivider />

<div class={css({ position: 'relative' })}>
  <div class={flex({ alignItems: 'center', paddingX: '16px' })}>
    <button
      class={css({
        position: 'relative',
        padding: '12px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: activeTab === 'publish' ? 'text.default' : 'text.muted',
        transition: 'common',
        _hover: {
          color: activeTab === 'publish' ? 'text.default' : 'text.subtle',
        },
        _after: {
          content: '""',
          position: 'absolute',
          bottom: '0',
          insetX: '0',
          height: '3px',
          backgroundColor: activeTab === 'publish' ? 'accent.brand.default' : 'transparent',
          transition: 'common',
        },
      })}
      onclick={() => (activeTab = 'publish')}
      type="button"
    >
      조회
    </button>
    <button
      class={css({
        position: 'relative',
        padding: '12px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: activeTab === 'share' ? 'text.default' : 'text.muted',
        transition: 'common',
        _hover: {
          color: activeTab === 'share' ? 'text.default' : 'text.subtle',
        },
        _after: {
          content: '""',
          position: 'absolute',
          bottom: '0',
          insetX: '0',
          height: '3px',
          backgroundColor: activeTab === 'share' ? 'accent.brand.default' : 'transparent',
          transition: 'common',
        },
      })}
      onclick={() => (activeTab = 'share')}
      type="button"
    >
      편집
    </button>
  </div>

  <div
    class={css({
      position: 'absolute',
      insetX: '0',
      bottom: '0',
      height: '1px',
      backgroundColor: 'border.default',
    })}
  ></div>
</div>

{#if activeTab === 'publish'}
  <div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>문서 조회 권한</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={BlendIcon} size={14} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>공개 범위</div>
        </div>

        <Select
          items={[
            {
              icon: LinkIcon,
              label: '링크가 있는 사람',
              description: '링크가 있는 누구나 볼 수 있어요.',
              value: EntityVisibility.UNLISTED,
            },
            {
              icon: LockIcon,
              label: '비공개',
              description: '나만 볼 수 있어요.',
              value: EntityVisibility.PRIVATE,
            },
          ]}
          values={$documents.map((d) => d.entity.visibility)}
          bind:value={form.fields.visibility}
        />
      </div>
    </div>
  </div>
{:else if activeTab === 'share'}
  <div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>문서 편집 권한</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={BlendIcon} size={14} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>편집 범위</div>
        </div>

        <Select
          items={[
            {
              icon: LinkIcon,
              label: '링크가 있는 사람',
              description: '링크가 있는 누구나 편집할 수 있어요.',
              value: EntityAvailability.UNLISTED,
            },
            {
              icon: LockIcon,
              label: '나만',
              description: '나만 편집할 수 있어요.',
              value: EntityAvailability.PRIVATE,
            },
          ]}
          values={$documents.map((d) => d.entity.availability)}
          bind:value={form.fields.availability}
        />
      </div>
    </div>
  </div>
{/if}
