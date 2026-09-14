<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { Img } from '$lib/components';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import ReorderableList from './ReorderableList.svelte';
  import type { DashboardLayout_SiteSettingsModal_CollectionEditor_collection$key } from '$mearie';

  type Props = {
    collection$key: DashboardLayout_SiteSettingsModal_CollectionEditor_collection$key;
    onclose: () => void;
  };

  let { collection$key, onclose }: Props = $props();

  const collection = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_CollectionEditor_collection on Collection {
        id
        name
        description

        cover {
          id
          ...Img_image
        }

        publications {
          id
          title
          state
          collectionOrder
        }
      }
    `),
    () => collection$key,
  );

  const [updateCollection] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_CollectionEditor_UpdateCollection_Mutation($input: UpdateCollectionInput!) {
        updateCollection(input: $input) {
          id
          name
          description

          cover {
            id
            ...Img_image
          }
        }
      }
    `),
  );

  const [deleteCollection, deleteResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_CollectionEditor_DeleteCollection_Mutation($input: DeleteCollectionInput!) {
        deleteCollection(input: $input) {
          id

          space {
            id

            collections {
              id
              name
            }
          }
        }
      }
    `),
  );

  const [movePublicationInCollection, moveResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_CollectionEditor_MovePublicationInCollection_Mutation(
        $input: MovePublicationInCollectionInput!
      ) {
        movePublicationInCollection(input: $input) {
          id
          collectionOrder

          collection {
            id

            publications {
              id
              title
              state
              collectionOrder
            }
          }
        }
      }
    `),
  );

  const STATE_LABELS = { SCHEDULED: '예약됨', UNPUBLISHED: '발행 취소됨' } as const;

  type UpdateCollectionFields = Omit<Parameters<typeof updateCollection>[0]['input'], 'collectionId'>;

  const save = async (input: UpdateCollectionFields, field: string) => {
    if (!SubscribeModal.gate('space_settings')) return false;

    try {
      await updateCollection({ input: { collectionId: collection.data.id, ...input } });
      mixpanel.track('update_collection', { field });
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  const nameForm = createForm({
    schema: z.object({
      name: z.string({ error: '시리즈 이름을 입력해주세요.' }).trim().min(1, '시리즈 이름을 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      const saved = await save({ name: data.name }, 'name');
      if (!saved) {
        nameForm.fields.name = collection.data.name;
      }
    },
    defaultValues: {
      name: collection.data.name,
    },
  });

  $effect(() => {
    void nameForm;
  });

  let description = $state(collection.data.description ?? '');
  let coverUploading = $state(false);

  const saveDescription = () => {
    const next = description.trim();
    if (next === (collection.data.description ?? '')) return;
    void save({ description: next || null }, 'description');
  };

  const handleCoverUpload = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      if (!file || !SubscribeModal.gate('space_settings')) return;

      coverUploading = true;
      try {
        const image = await uploadBlobAsImage(file);
        await save({ coverId: image.id }, 'cover');
      } catch (err) {
        Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      } finally {
        coverUploading = false;
      }
    });

    input.click();
  };

  const items = $derived(
    collection.data.publications.map((publication) => ({ id: publication.id, order: publication.collectionOrder ?? '' })),
  );

  const reorder = async (move: { id: string; lowerOrder: string | null; upperOrder: string | null }) => {
    if (moveResult.loading || !SubscribeModal.gate('space_settings')) return false;

    try {
      await movePublicationInCollection({ input: { publicationId: move.id, lowerOrder: move.lowerOrder, upperOrder: move.upperOrder } });
      mixpanel.track('move_publication_in_collection');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  const confirmDelete = () => {
    if (!SubscribeModal.gate('space_settings')) return;

    Dialog.confirm({
      title: '시리즈를 삭제하시겠어요?',
      message: '시리즈에 속한 글은 시리즈 없음이 되고, 글은 그대로 발행돼요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        try {
          await deleteCollection({ input: { collectionId: collection.data.id } });
          mixpanel.track('delete_collection');
          onclose();
        } catch (err) {
          Toast.error(publicationErrorMessage(publicationErrorCode(err)));
        }
      },
    });
  };
</script>

<div class={flex({ flexDirection: 'column', gap: '12px', paddingX: '20px', paddingY: '16px', backgroundColor: 'surface.canvas' })}>
  <div class={flex({ alignItems: 'center', gap: '12px' })}>
    <button
      class={center({
        flexShrink: '0',
        width: '64px',
        height: '36px',
        borderWidth: collection.data.cover ? '0' : '1px',
        borderStyle: 'dashed',
        borderRadius: '6px',
        color: 'text.muted',
        overflow: 'hidden',
        _hover: { backgroundColor: 'surface.hover' },
      })}
      disabled={coverUploading}
      onclick={handleCoverUpload}
      type="button"
      use:tooltip={{ message: '표지', placement: 'top' }}
    >
      {#if collection.data.cover}
        <Img
          style={css.raw({ width: '64px', height: '36px', objectFit: 'cover' })}
          alt="표지"
          image$key={collection.data.cover}
          size={128}
        />
      {:else if coverUploading}
        <span class={css({ fontSize: '10px' })}>...</span>
      {:else}
        <Icon icon={ImageIcon} size={14} />
      {/if}
    </button>

    <TextInput
      style={css.raw({ flex: '1', height: '32px', fontSize: '13px' })}
      onblur={() => {
        if (nameForm.state.isDirty) {
          nameForm.handleSubmit();
        }
      }}
      placeholder="시리즈 이름"
      bind:value={nameForm.fields.name}
    />

    {#if collection.data.cover}
      <button
        class={center({
          size: '24px',
          borderRadius: '4px',
          color: 'text.muted',
          _hover: { backgroundColor: 'surface.hover', color: 'danger.default' },
        })}
        onclick={() => save({ coverId: null }, 'cover')}
        type="button"
        use:tooltip={{ message: '삭제', placement: 'top' }}
      >
        <Icon icon={Trash2Icon} size={14} />
      </button>
    {/if}
  </div>

  {#if nameForm.errors.name}
    <p class={css({ fontSize: '12px', color: 'danger.default' })}>{nameForm.errors.name}</p>
  {/if}

  <textarea
    class={css({
      width: 'full',
      minHeight: '56px',
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
    aria-label="설명"
    onblur={saveDescription}
    placeholder="설명"
    bind:value={description}></textarea>

  {#if items.length > 0}
    <ReorderableList disabled={moveResult.loading} {items} onmove={reorder}>
      {#snippet row(id)}
        {@const publication = collection.data.publications.find((candidate) => candidate.id === id)}

        {#if publication}
          <div class={flex({ alignItems: 'center', gap: '6px', minWidth: '0', height: '36px', paddingRight: '8px' })}>
            <span class={css({ fontSize: '13px', color: 'text.default', lineClamp: '1', wordBreak: 'break-all' })}>
              {publication.title ?? '(제목 없음)'}
            </span>
            {#if publication.state !== 'PUBLISHED'}
              <span class={css({ flexShrink: '0', fontSize: '11px', color: 'text.hint' })}>{STATE_LABELS[publication.state]}</span>
            {/if}
          </div>
        {/if}
      {/snippet}
    </ReorderableList>
  {:else}
    <p class={css({ fontSize: '12px', color: 'text.hint' })}>아직 발행한 글이 없어요</p>
  {/if}

  <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
    <Button loading={deleteResult.loading} onclick={confirmDelete} size="sm" variant="ghost">
      <Icon icon={Trash2Icon} size={14} />
      삭제
    </Button>
    <Button onclick={onclose} size="sm" variant="secondary">닫기</Button>
  </div>
</div>
