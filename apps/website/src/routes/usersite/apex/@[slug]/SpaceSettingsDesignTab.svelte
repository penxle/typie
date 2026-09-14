<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { SpaceDateDisplay } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { Select } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import type { UsersiteSpace_SpaceSettingsDesignTab_space$key } from '$mearie';

  type Props = {
    space$key: UsersiteSpace_SpaceSettingsDesignTab_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteSpace_SpaceSettingsDesignTab_space on Space {
        id
        dateDisplay
      }
    `),
    () => space$key,
  );

  const [updateSpace] = createMutation(
    graphql(`
      mutation UsersiteSpace_SpaceSettingsDesignTab_UpdateSpace_Mutation($input: UpdateSpaceInput!) {
        updateSpace(input: $input) {
          id
          dateDisplay
        }
      }
    `),
  );

  const setDateDisplay = async (dateDisplay: SpaceDateDisplay) => {
    try {
      await updateSpace({ input: { spaceId: space.data.id, dateDisplay } });
      cache.invalidate({ __typename: 'Query', $field: 'spaceView' });
      mixpanel.track('update_space', { field: 'dateDisplay', via: 'space_page' });
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>디자인</h1>
  </div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        글 목록에 표시할 날짜
      {/snippet}
      {#snippet value()}
        <Select
          items={[
            { label: '발행 시각', value: SpaceDateDisplay.PUBLISHED_AT },
            { label: '마지막 수정 시각', value: SpaceDateDisplay.UPDATED_AT },
            { label: '미표시', value: SpaceDateDisplay.NONE },
          ]}
          onselect={setDateDisplay}
          value={space.data.dateDisplay}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
