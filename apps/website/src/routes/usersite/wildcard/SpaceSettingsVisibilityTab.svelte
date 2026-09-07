<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Switch } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { SettingsCard, SettingsRow } from '$lib/components';
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
        }
      }
    `),
  );

  let allowIndexing = $state(space.data.allowIndexing);

  $effect(() => {
    allowIndexing = space.data.allowIndexing;
  });

  const setAllowIndexing = async (checked: boolean) => {
    allowIndexing = checked;
    try {
      await updateSpace({ input: { spaceId: space.data.id, allowIndexing: checked } });
      cache.invalidate({ __typename: 'Query', $field: 'spaceView' });
      mixpanel.track('update_space', { field: 'allowIndexing', via: 'space_page' });
      Toast.success('스페이스 설정이 업데이트됐어요.');
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      await tick();
      allowIndexing = space.data.allowIndexing;
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
