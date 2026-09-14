<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, TextInput } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import ImageIcon from '~icons/lucide/image';
  import PencilIcon from '~icons/lucide/pencil';
  import PlusIcon from '~icons/lucide/plus';
  import TrashIcon from '~icons/lucide/trash';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { LoadableImg, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import type { UsersiteWildcard_SpaceSettingsCollectionsTab_space$key } from '$mearie';

  type Props = {
    space$key: UsersiteWildcard_SpaceSettingsCollectionsTab_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteWildcard_SpaceSettingsCollectionsTab_space on Space {
        id

        collections {
          id
          name
          description

          cover {
            id
          }

          publications {
            id
          }
        }
      }
    `),
    () => space$key,
  );

  const [createCollection] = createMutation(
    graphql(`
      mutation UsersiteWildcard_SpaceSettingsCollectionsTab_CreateCollection_Mutation($input: CreateCollectionInput!) {
        createCollection(input: $input) {
          id

          space {
            id

            collections {
              id
              name
              description

              cover {
                id
              }

              publications {
                id
              }
            }
          }
        }
      }
    `),
  );

  const [updateCollection] = createMutation(
    graphql(`
      mutation UsersiteWildcard_SpaceSettingsCollectionsTab_UpdateCollection_Mutation($input: UpdateCollectionInput!) {
        updateCollection(input: $input) {
          id
          name
          description

          cover {
            id
          }
        }
      }
    `),
  );

  const [deleteCollection] = createMutation(
    graphql(`
      mutation UsersiteWildcard_SpaceSettingsCollectionsTab_DeleteCollection_Mutation($input: DeleteCollectionInput!) {
        deleteCollection(input: $input) {
          id

          space {
            id

            collections {
              id
            }
          }
        }
      }
    `),
  );

  type Collection = (typeof space.data.collections)[number];

  let creatingNew = $state(false);
  let editingId = $state<string | null>(null);
  let formName = $state('');
  let formDescription = $state('');
  let formCoverId = $state<string | null>(null);
  let formError = $state('');
  let saving = $state(false);
  let coverUploading = $state(false);

  const refresh = () => {
    cache.invalidate({ __typename: 'Query', $field: 'spaceView' });
  };

  const resetForm = () => {
    formName = '';
    formDescription = '';
    formCoverId = null;
    formError = '';
  };

  const startCreate = () => {
    editingId = null;
    creatingNew = true;
    resetForm();
  };

  const startEdit = (collection: Collection) => {
    creatingNew = false;
    editingId = collection.id;
    formName = collection.name;
    formDescription = collection.description ?? '';
    formCoverId = collection.cover?.id ?? null;
    formError = '';
  };

  const cancelForm = () => {
    creatingNew = false;
    editingId = null;
    resetForm();
  };

  const handleSave = async () => {
    const name = formName.trim();
    if (!name) {
      formError = '시리즈 이름을 입력해 주세요.';
      return;
    }
    if (saving) return;

    const description = formDescription.trim() || null;

    saving = true;
    try {
      if (creatingNew) {
        await createCollection({ input: { spaceId: space.data.id, name, description, coverId: formCoverId } });
        mixpanel.track('create_collection', { via: 'space_page' });
      } else if (editingId) {
        await updateCollection({ input: { collectionId: editingId, name, description, coverId: formCoverId } });
        mixpanel.track('update_collection', { via: 'space_page' });
      }
      refresh();
      cancelForm();
    } catch (err) {
      formError = publicationErrorMessage(publicationErrorCode(err));
    } finally {
      saving = false;
    }
  };

  const handleCoverUpload = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      if (!file) return;

      coverUploading = true;
      try {
        const image = await uploadBlobAsImage(file);
        formCoverId = image.id;
      } catch (err) {
        Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      } finally {
        coverUploading = false;
      }
    });

    input.click();
  };

  const handleDelete = (collection: Collection) => {
    Dialog.confirm({
      title: '시리즈 삭제',
      message: `"${collection.name}" 시리즈를 삭제하시겠어요? 속해있던 글들은 삭제되지 않고 시리즈에서만 사라져요.`,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        try {
          await deleteCollection({ input: { collectionId: collection.id } });
          refresh();
          mixpanel.track('delete_collection', { via: 'space_page' });
        } catch (err) {
          Toast.error(publicationErrorMessage(publicationErrorCode(err)));
        }
      },
    });
  };
</script>

{#snippet form()}
  <div class={css({ paddingX: '20px', paddingY: '16px' })}>
    <div class={flex({ alignItems: 'flex-start', gap: '12px', marginBottom: '12px' })}>
      <div class={flex({ flexShrink: '0', alignItems: 'center', gap: '4px' })}>
        {#if formCoverId}
          <button class={css({ cursor: 'pointer' })} disabled={coverUploading} onclick={handleCoverUpload} type="button">
            <LoadableImg
              id={formCoverId}
              style={css.raw({ size: '72px', borderRadius: '6px', objectFit: 'cover' })}
              alt="표지"
              size={128}
            />
          </button>
          <button
            class={center({
              size: '24px',
              borderRadius: '4px',
              color: 'text.muted',
              _hover: { backgroundColor: 'surface.hover', color: 'danger.default' },
            })}
            onclick={() => (formCoverId = null)}
            type="button"
            use:tooltip={{ message: '삭제', placement: 'top' }}
          >
            <Icon icon={Trash2Icon} size={14} />
          </button>
        {:else}
          <button
            class={center({
              size: '72px',
              borderWidth: '1px',
              borderStyle: 'dashed',
              borderRadius: '6px',
              color: 'text.muted',
              _hover: { backgroundColor: 'surface.hover' },
            })}
            disabled={coverUploading}
            onclick={handleCoverUpload}
            type="button"
            use:tooltip={{ message: '표지', placement: 'top' }}
          >
            {#if coverUploading}
              <span class={css({ fontSize: '10px' })}>...</span>
            {:else}
              <Icon icon={ImageIcon} size={14} />
            {/if}
          </button>
        {/if}
      </div>

      <div class={flex({ flex: '1', flexDirection: 'column', gap: '8px' })}>
        <TextInput style={css.raw({ width: 'full' })} autofocus placeholder="시리즈 이름" size="sm" bind:value={formName} />
        <TextInput style={css.raw({ width: 'full' })} placeholder="설명 (선택)" size="sm" bind:value={formDescription} />
      </div>
    </div>

    <div class={flex({ justifyContent: 'flex-end', gap: '8px' })}>
      <Button onclick={cancelForm} size="sm" variant="secondary">취소</Button>
      <Button loading={saving} onclick={handleSave} size="sm" variant="primary">저장</Button>
    </div>
    {#if formError}
      <p class={css({ fontSize: '12px', color: 'danger.default', marginTop: '8px' })}>{formError}</p>
    {/if}
  </div>
{/snippet}

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '4px' })}>
      <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>시리즈</h1>
      <button
        class={flex({
          alignItems: 'center',
          gap: '6px',
          borderRadius: '6px',
          paddingX: '12px',
          paddingY: '6px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.muted',
          transition: 'common',
          _hover: { backgroundColor: 'surface.hover' },
        })}
        onclick={() => startCreate()}
        type="button"
      >
        <Icon style={css.raw({ color: 'text.muted' })} icon={PlusIcon} size={14} />
        <span>추가</span>
      </button>
    </div>
    <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>
      글을 순서 있는 묶음으로 모아요. 독자는 시리즈 페이지에서 이전 글·다음 글로 이어 읽을 수 있어요.
    </p>
  </div>

  <div>
    {#if space.data.collections.length > 0}
      <SettingsCard>
        {#each space.data.collections as collection, index (collection.id)}
          {#if index > 0}
            <SettingsDivider />
          {/if}
          {#if editingId === collection.id}
            {@render form()}
          {:else}
            <SettingsRow>
              {#snippet label()}
                <div class={flex({ alignItems: 'center', gap: '8px' })}>
                  {#if collection.cover}
                    <LoadableImg
                      id={collection.cover.id}
                      style={css.raw({ size: '24px', borderRadius: '4px', objectFit: 'cover' })}
                      alt="표지"
                      size={64}
                    />
                  {/if}
                  <span>{collection.name}</span>
                  <span
                    class={css({
                      flexShrink: '0',
                      borderRadius: '4px',
                      paddingX: '6px',
                      paddingY: '2px',
                      fontSize: '11px',
                      fontWeight: 'medium',
                      color: 'text.muted',
                      backgroundColor: 'surface.inset',
                    })}
                  >
                    글 {collection.publications.length}개
                  </span>
                </div>
              {/snippet}
              {#snippet description()}
                {collection.description ?? ''}
              {/snippet}
              {#snippet value()}
                <div class={flex({ alignItems: 'center', gap: '8px' })}>
                  <button
                    class={css({
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      size: '28px',
                      borderRadius: '6px',
                      color: 'text.muted',
                      transition: 'common',
                      _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
                    })}
                    onclick={() => startEdit(collection)}
                    type="button"
                  >
                    <Icon icon={PencilIcon} size={14} />
                  </button>
                  <button
                    class={css({
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      size: '28px',
                      borderRadius: '6px',
                      color: 'text.muted',
                      transition: 'common',
                      _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
                    })}
                    onclick={() => handleDelete(collection)}
                    type="button"
                  >
                    <Icon icon={TrashIcon} size={14} />
                  </button>
                </div>
              {/snippet}
            </SettingsRow>
          {/if}
        {/each}
      </SettingsCard>
    {:else if !creatingNew}
      <SettingsCard>
        <div class={css({ padding: '20px', fontSize: '13px', color: 'text.hint', textAlign: 'center' })}>아직 만든 시리즈가 없어요.</div>
      </SettingsCard>
    {/if}

    {#if creatingNew}
      <div class={css({ marginTop: space.data.collections.length > 0 ? '12px' : '0' })}>
        <SettingsCard>
          {@render form()}
        </SettingsCard>
      </div>
    {/if}
  </div>
</div>
