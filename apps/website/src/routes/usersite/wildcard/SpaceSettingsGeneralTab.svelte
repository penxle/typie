<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import Trash2Icon from '~icons/lucide/trash-2';
  import UploadIcon from '~icons/lucide/upload';
  import { env } from '$env/dynamic/public';
  import { Img, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import SpaceSlugModal from './SpaceSlugModal.svelte';
  import type { UsersiteWildcard_SpaceSettingsGeneralTab_space$key } from '$mearie';

  type Props = {
    space$key: UsersiteWildcard_SpaceSettingsGeneralTab_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteWildcard_SpaceSettingsGeneralTab_space on Space {
        id
        name
        slug

        logo {
          id
          ...Img_image
        }

        site {
          id

          logo {
            id
            ...Img_image
          }
        }
      }
    `),
    () => space$key,
  );

  const [updateSpace] = createMutation(
    graphql(`
      mutation UsersiteWildcard_SpaceSettingsGeneralTab_UpdateSpace_Mutation($input: UpdateSpaceInput!) {
        updateSpace(input: $input) {
          id
          name
          slug
          url

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
      mutation UsersiteWildcard_SpaceSettingsGeneralTab_DeleteSpace_Mutation($input: DeleteSpaceInput!) {
        deleteSpace(input: $input) {
          id
        }
      }
    `),
  );

  type UpdateSpaceFields = Omit<Parameters<typeof updateSpace>[0]['input'], 'spaceId'>;

  const refresh = () => {
    cache.invalidate({ __typename: 'Query', $field: 'spaceView' });
  };

  const save = async (input: UpdateSpaceFields, field: string) => {
    try {
      const resp = await updateSpace({ input: { spaceId: space.data.id, ...input } });
      refresh();
      mixpanel.track('update_space', { field, via: 'space_page' });
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return resp.updateSpace;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return null;
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

  let slugModalOpen = $state(false);

  let logoUploading = $state(false);

  const handleLogoChange = async (file: File) => {
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
          mixpanel.track('delete_space', { via: 'space_page' });
          location.assign(env.PUBLIC_WEBSITE_URL);
        } catch (err) {
          Toast.error(publicationErrorMessage(publicationErrorCode(err)));
        }
      },
    });
  };
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>일반</h1>
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
              image$key={space.data.logo ?? space.data.site.logo}
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
          onkeydown={(e) => {
            if (e.key === 'Enter' && !e.isComposing && nameForm.state.isDirty) {
              e.preventDefault();
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
          style={css.raw({ width: '[280px]', height: '32px', fontSize: '13px', cursor: 'pointer', '& > input': { cursor: 'pointer' } })}
          onclick={() => {
            slugModalOpen = true;
          }}
          readonly
          rightItemAttached
          value={space.data.slug}
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
    </SettingsRow>
  </SettingsCard>

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

<SpaceSlugModal slug={space.data.slug} spaceId={space.data.id} bind:open={slugModalOpen} />
