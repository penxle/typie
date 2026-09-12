<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { SpaceDateDisplay } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { siteSchema } from '@typie/lib/validation';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner, Select, Switch, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { z } from 'zod';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import Trash2Icon from '~icons/lucide/trash-2';
  import UploadIcon from '~icons/lucide/upload';
  import { replaceState } from '$app/navigation';
  import { env } from '$env/dynamic/public';
  import { Img, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import SpaceCollectionsSection from './SpaceCollectionsSection.svelte';
  import SpaceLinksEditor from './SpaceLinksEditor.svelte';
  import SpacePinnedSection from './SpacePinnedSection.svelte';
  import type { DashboardLayout_SiteSettingsModal_SpaceTab_site$key, DashboardLayout_SiteSettingsModal_SpaceTab_space$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_SiteSettingsModal_SpaceTab_site$key;
    space$key: DashboardLayout_SiteSettingsModal_SpaceTab_space$key;
  };

  let { site$key, space$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_SpaceTab_site on Site {
        id

        logo {
          id
          ...Img_image
        }
      }
    `),
    () => site$key,
  );

  const space = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_SpaceTab_space on Space {
        id
        name
        slug
        url
        description
        allowIndexing
        dateDisplay

        links {
          label
          url
        }

        logo {
          id
          ...Img_image
        }
      }
    `),
    () => space$key,
  );

  const [updateSpace] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_SpaceTab_UpdateSpace_Mutation($input: UpdateSpaceInput!) {
        updateSpace(input: $input) {
          id
          name
          slug
          url
          description
          allowIndexing
          dateDisplay

          links {
            label
            url
          }

          logo {
            id
            ...Img_image
          }
        }
      }
    `),
  );

  const [deleteSpace, deleteSpaceResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_SpaceTab_DeleteSpace_Mutation($input: DeleteSpaceInput!) {
        deleteSpace(input: $input) {
          id
        }
      }
    `),
  );

  const detail = createQuery(
    graphql(`
      query DashboardLayout_SiteSettingsModal_SpaceTab_Detail_Query($siteId: ID!) {
        site(siteId: $siteId) {
          id

          spaces {
            id

            ...DashboardLayout_SiteSettingsModal_SpacePinnedSection_space
            ...DashboardLayout_SiteSettingsModal_SpaceCollectionsSection_space
          }
        }
      }
    `),
    () => ({ siteId: site.data.id }),
  );

  const detailSpace = $derived(detail.data?.site.spaces.find((candidate) => candidate.id === space.data.id));

  $effect(() => {
    if (detail.error) {
      Toast.error(publicationErrorMessage(publicationErrorCode(detail.error)));
    }
  });

  type UpdateSpaceFields = Omit<Parameters<typeof updateSpace>[0]['input'], 'spaceId'>;

  const save = async (input: UpdateSpaceFields, field: string) => {
    if (!SubscribeModal.gate('space_settings')) return false;

    try {
      await updateSpace({ input: { spaceId: space.data.id, ...input } });
      mixpanel.track('update_space', { field });
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  const nameForm = createForm({
    schema: z.object({
      name: z.string({ error: '스페이스 이름을 입력해주세요.' }).trim().min(1, '스페이스 이름을 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      const saved = await save({ name: data.name }, 'name');
      if (!saved) {
        nameForm.fields.name = space.data.name;
      }
    },
    defaultValues: {
      name: space.data.name,
    },
  });

  const slugForm = createForm({
    schema: z.object({
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      if (!SubscribeModal.gate('space_settings')) {
        slugForm.fields.slug = space.data.slug;
        return;
      }

      await updateSpace({ input: { spaceId: space.data.id, slug: data.slug } });
      mixpanel.track('update_space', { field: 'slug' });
      Toast.success('스페이스 주소가 변경됐어요.');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'space_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 스페이스 주소예요.');
      }

      if (error instanceof TypieError) {
        throw new FormError('slug', publicationErrorMessage(error.code));
      }
    },
    defaultValues: {
      slug: space.data.slug,
    },
  });

  let description = $state(space.data.description ?? '');
  let logoUploading = $state(false);
  let allowIndexing = $state(space.data.allowIndexing);

  $effect(() => {
    allowIndexing = space.data.allowIndexing;
  });

  const setAllowIndexing = async (checked: boolean) => {
    allowIndexing = checked;

    if (!(await save({ allowIndexing: checked }, 'allowIndexing'))) {
      await tick();
      allowIndexing = space.data.allowIndexing;
    }
  };

  const saveDescription = () => {
    const next = description.trim();
    if (next === (space.data.description ?? '')) return;
    void save({ description: next || null }, 'description');
  };

  const handleLogoChange = async (file: File) => {
    if (!SubscribeModal.gate('space_settings')) {
      return;
    }

    logoUploading = true;
    try {
      const resp = await uploadBlobAsImage(file, {
        resize: { width: 512, height: 512, fit: 'cover', withoutEnlargement: true },
        format: 'png',
      });
      await save({ logoId: resp.id }, 'logo');
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    } finally {
      logoUploading = false;
    }
  };

  const confirmDelete = () => {
    Dialog.confirm({
      title: '스페이스를 삭제하시겠어요?',
      message: '스페이스의 글이 모두 발행 취소되고, 이 주소는 더 쓸 수 없어요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        try {
          await deleteSpace({ input: { spaceId: space.data.id } });
          cache.invalidate({ __typename: 'Site', id: site.data.id, $field: 'spaces' });
          mixpanel.track('delete_space');
          Toast.success('스페이스가 삭제됐어요.');
          replaceState('', { shallowRoute: '/site-settings/general' });
        } catch (err) {
          Toast.error(publicationErrorMessage(publicationErrorCode(err)));
        }
      },
    });
  };
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between', gap: '16px', marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', lineClamp: '1', wordBreak: 'break-all' })}>
      {space.data.name}
    </h1>
    <a
      class={flex({
        alignItems: 'center',
        gap: '4px',
        flexShrink: '0',
        fontSize: '13px',
        color: 'text.muted',
        _hover: { color: 'text.default' },
      })}
      href={space.data.url}
      rel="noopener noreferrer"
      target="_blank"
    >
      스페이스 열기
      <Icon icon={ArrowUpRightIcon} size={12} />
    </a>
  </div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        로고
      {/snippet}
      {#snippet description()}
        없으면 작업실 로고를 써요.
      {/snippet}
      {#snippet value()}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <label class={cx('group', center({ position: 'relative', size: '32px', cursor: 'pointer' }))}>
            <Img
              style={css.raw({ size: '32px', borderRadius: '4px' })}
              alt={space.data.name}
              image$key={space.data.logo ?? site.data.logo}
              size={64}
            />
            <div
              class={css({
                display: 'none',
                _groupHover: {
                  position: 'absolute',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  borderRadius: '4px',
                  size: 'full',
                  backgroundColor: 'surface.inverse/70',
                  color: 'text.on.inverse',
                },
              })}
            >
              <Icon icon={UploadIcon} size={14} />
            </div>
            <input
              accept="image/*"
              disabled={logoUploading}
              hidden
              onchange={async (event) => {
                const file = event.currentTarget.files?.[0];
                event.currentTarget.value = '';
                if (!file) return;
                await handleLogoChange(file);
              }}
              type="file"
            />
          </label>

          {#if space.data.logo}
            <button
              class={center({
                size: '24px',
                borderRadius: '4px',
                color: 'text.muted',
                _hover: { backgroundColor: 'surface.hover', color: 'danger.default' },
              })}
              onclick={() => save({ logoId: null }, 'logo')}
              type="button"
              use:tooltip={{ message: '삭제', placement: 'top' }}
            >
              <Icon icon={Trash2Icon} size={14} />
            </button>
          {/if}
        </div>
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow>
      {#snippet label()}
        이름
      {/snippet}
      {#snippet value()}
        <TextInput
          style={css.raw({ width: '[200px]', height: '32px', fontSize: '13px' })}
          onblur={() => {
            if (nameForm.state.isDirty) {
              nameForm.handleSubmit();
            }
          }}
          bind:value={nameForm.fields.name}
        />
      {/snippet}
      {#snippet error()}
        {#if nameForm.errors.name}
          <p class={css({ fontSize: '12px', color: 'danger.default', textAlign: 'right' })}>{nameForm.errors.name}</p>
        {/if}
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow>
      {#snippet label()}
        주소
      {/snippet}
      {#snippet value()}
        <TextInput
          style={css.raw({ width: '[280px]', height: '32px', fontSize: '13px' })}
          onblur={() => {
            if (slugForm.state.isDirty) {
              slugForm.handleSubmit();
            }
          }}
          rightItemAttached
          bind:value={slugForm.fields.slug}
        >
          {#snippet rightItem()}
            <span
              class={css({
                fontSize: '13px',
                color: 'text.muted',
                backgroundColor: 'surface.inset',
                paddingX: '12px',
                height: 'full',
                display: 'flex',
                alignItems: 'center',
              })}
            >
              .{env.PUBLIC_USERSITE_HOST}
            </span>
          {/snippet}
        </TextInput>
      {/snippet}
      {#snippet error()}
        {#if slugForm.errors.slug}
          <p class={css({ fontSize: '12px', color: 'danger.default', textAlign: 'right' })}>{slugForm.errors.slug}</p>
        {/if}
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow vertical>
      {#snippet label()}
        소개
      {/snippet}
      {#snippet value()}
        <textarea
          class={css({
            width: 'full',
            minHeight: '72px',
            paddingX: '12px',
            paddingY: '8px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '6px',
            fontSize: '13px',
            lineHeight: '[1.5]',
            color: 'text.default',
            backgroundColor: 'surface.default',
            resize: 'none',
            transition: 'common',
            _hover: { borderColor: 'border.emphasis' },
            _focus: { outline: 'none', borderColor: 'accent.default' },
            _placeholder: { color: 'text.hint' },
          })}
          aria-label="소개"
          onblur={saveDescription}
          bind:value={description}></textarea>
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow vertical>
      {#snippet label()}
        링크
      {/snippet}
      {#snippet value()}
        <SpaceLinksEditor links={space.data.links} onsave={(links) => save({ links }, 'links')} />
      {/snippet}
    </SettingsRow>
  </SettingsCard>

  <div class={css({ marginTop: '40px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>공개</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          검색 엔진에 노출
        {/snippet}
        {#snippet value()}
          <Switch bind:checked={() => allowIndexing, (checked) => void setAllowIndexing(checked)} />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          날짜 표시
        {/snippet}
        {#snippet value()}
          <Select
            items={[
              { label: '없음', value: SpaceDateDisplay.NONE },
              { label: '발행일', value: SpaceDateDisplay.PUBLISHED_AT },
              { label: '갱신일', value: SpaceDateDisplay.UPDATED_AT },
            ]}
            onselect={(value) => save({ dateDisplay: value }, 'dateDisplay')}
            value={space.data.dateDisplay}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  {#if detail.loading && !detail.data}
    <div class={flex({ justifyContent: 'center', paddingY: '24px' })}>
      <RingSpinner style={css.raw({ size: '20px', color: 'text.muted' })} />
    </div>
  {:else if !detail.error && detailSpace}
    <SpacePinnedSection space$key={detailSpace} />
    <SpaceCollectionsSection space$key={detailSpace} />
  {/if}

  <div class={css({ marginTop: '40px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>스페이스 삭제</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          스페이스 삭제
        {/snippet}
        {#snippet description()}
          스페이스의 글이 모두 발행 취소되고, 이 주소는 더 쓸 수 없어요.
        {/snippet}
        {#snippet value()}
          <Button loading={deleteSpaceResult.loading} onclick={confirmDelete} size="sm" variant="ghost">삭제하기</Button>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>
