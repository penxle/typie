<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import CollectionEditor from './CollectionEditor.svelte';
  import type { DashboardLayout_SiteSettingsModal_SpaceCollectionsSection_space$key } from '$mearie';

  type Props = {
    space$key: DashboardLayout_SiteSettingsModal_SpaceCollectionsSection_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_SpaceCollectionsSection_space on Space {
        id

        collections {
          id
          name

          publications {
            id
          }

          ...DashboardLayout_SiteSettingsModal_CollectionEditor_collection
        }
      }
    `),
    () => space$key,
  );

  const [createCollection, createResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_SpaceCollectionsSection_CreateCollection_Mutation($input: CreateCollectionInput!) {
        createCollection(input: $input) {
          id
          name

          space {
            id

            collections {
              id
              name

              publications {
                id
              }

              ...DashboardLayout_SiteSettingsModal_CollectionEditor_collection
            }
          }
        }
      }
    `),
  );

  let editingId = $state<string | null>(null);
  let newName = $state('');

  const create = async () => {
    const name = newName.trim();
    if (!name || createResult.loading || !SubscribeModal.gate('space_settings')) return;

    try {
      const resp = await createCollection({ input: { spaceId: space.data.id, name } });
      newName = '';
      editingId = resp.createCollection.id;
      mixpanel.track('create_collection', { via: 'space_settings' });
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
    }
  };
</script>

<div class={css({ marginTop: '40px' })}>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>시리즈</h2>

  <SettingsCard>
    {#each space.data.collections as collection (collection.id)}
      <SettingsRow>
        {#snippet label()}
          {collection.name}
        {/snippet}
        {#snippet description()}
          {collection.publications.length}개의 글
        {/snippet}
        {#snippet value()}
          <Button onclick={() => (editingId = editingId === collection.id ? null : collection.id)} size="sm" variant="ghost">
            {editingId === collection.id ? '닫기' : '편집'}
          </Button>
        {/snippet}
      </SettingsRow>

      {#if editingId === collection.id}
        <CollectionEditor collection$key={collection} onclose={() => (editingId = null)} />
      {/if}

      <SettingsDivider />
    {/each}

    <SettingsRow>
      {#snippet label()}
        새 시리즈
      {/snippet}
      {#snippet value()}
        <div class={flex({ alignItems: 'center', gap: '6px' })}>
          <TextInput
            style={css.raw({ width: '[200px]', height: '32px', fontSize: '13px' })}
            onkeydown={(e) => {
              if (e.key === 'Enter' && !e.isComposing) {
                e.preventDefault();
                void create();
              }
            }}
            placeholder="시리즈 이름"
            bind:value={newName}
          />
          <Button disabled={newName.trim().length === 0} loading={createResult.loading} onclick={create} size="sm" variant="secondary">
            만들기
          </Button>
        </div>
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
