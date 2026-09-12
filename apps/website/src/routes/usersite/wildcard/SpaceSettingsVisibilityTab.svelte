<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Switch } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import type { UsersiteWildcard_SpaceSettingsVisibilityTab_space$key } from '$mearie';

  type Props = {
    space$key: UsersiteWildcard_SpaceSettingsVisibilityTab_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteWildcard_SpaceSettingsVisibilityTab_space on Space {
        id
        allowIndexing
        allowDiscovery
      }
    `),
    () => space$key,
  );

  const [updateSpace] = createMutation(
    graphql(`
      mutation UsersiteWildcard_SpaceSettingsVisibilityTab_UpdateSpace_Mutation($input: UpdateSpaceInput!) {
        updateSpace(input: $input) {
          id
          allowIndexing
          allowDiscovery
        }
      }
    `),
  );

  let allowIndexing = $state(space.data.allowIndexing);
  let allowDiscovery = $state(space.data.allowDiscovery);

  $effect(() => {
    allowIndexing = space.data.allowIndexing;
    allowDiscovery = space.data.allowDiscovery;
  });

  const save = async (input: { allowIndexing?: boolean; allowDiscovery?: boolean }, field: string) => {
    try {
      await updateSpace({ input: { spaceId: space.data.id, ...input } });
      cache.invalidate({ __typename: 'Query', $field: 'spaceView' });
      mixpanel.track('update_space', { field, via: 'space_page' });
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  const setAllowIndexing = async (checked: boolean) => {
    allowIndexing = checked;
    if (!(await save({ allowIndexing: checked }, 'allowIndexing'))) {
      await tick();
      allowIndexing = space.data.allowIndexing;
    }
  };

  const setAllowDiscovery = async (checked: boolean) => {
    allowDiscovery = checked;
    if (!(await save({ allowDiscovery: checked }, 'allowDiscovery'))) {
      await tick();
      allowDiscovery = space.data.allowDiscovery;
    }
  };
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>노출</h1>
  </div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        타이피 스퀘어에 노출
      {/snippet}
      {#snippet description()}
        {allowIndexing ? '끄면 타이피 스퀘어 피드 및 검색에 스페이스와 글이 나오지 않아요.' : '검색 엔진에 노출을 켜야 쓸 수 있어요'}
      {/snippet}
      {#snippet value()}
        <Switch
          disabled={!allowIndexing}
          bind:checked={() => allowIndexing && allowDiscovery, (checked) => void setAllowDiscovery(checked)}
        />
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow>
      {#snippet label()}
        검색 엔진에 노출
      {/snippet}
      {#snippet description()}
        끄면 검색 엔진이 스페이스와 글을 색인하지 않도록 요청해요.
      {/snippet}
      {#snippet value()}
        <Switch bind:checked={() => allowIndexing, (checked) => void setAllowIndexing(checked)} />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
