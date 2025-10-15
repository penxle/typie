<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_FontTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_FontTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_FontTab_user on User {
        id

        fontFamilies {
          id
          name

          fonts {
            id
            weight
          }
        }
      }
    `),
  );

  const archiveFont = graphql(`
    mutation DashboardLayout_PreferenceModal_FontTab_ArchiveFont_Mutation($input: ArchiveFontInput!) {
      archiveFont(input: $input) {
        id
      }
    }
  `);
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>폰트</h1>
  </div>

  <!-- Font Management Section -->
  <div>
    {#if $user.fontFamilies.length > 0}
      <SettingsCard>
        {#each $user.fontFamilies as { id: familyId, name, fonts }, familyIndex (familyId)}
          {#each fonts as { id, weight }, fontIndex (id)}
            {#if familyIndex > 0 || fontIndex > 0}
              <SettingsDivider />
            {/if}
            <SettingsRow>
              {#snippet label()}
                <span style:font-family={familyId} style:font-weight={weight}>
                  {name}
                </span>
                <span class={css({ color: 'text.subtle' })}>
                  ({name})
                </span>
              {/snippet}
              {#snippet description()}
                굵기: {weight}
              {/snippet}
              {#snippet value()}
                <Button
                  onclick={() => {
                    Dialog.confirm({
                      title: '폰트 삭제',
                      message: `"${name}" 폰트를 삭제하시겠어요?`,
                      action: 'danger',
                      actionLabel: '삭제',
                      actionHandler: async () => {
                        await archiveFont({ fontId: id });
                        cache.invalidate({ __typename: 'User', id: $user.id, field: 'fontFamilies' });
                      },
                    });
                  }}
                  size="sm"
                  variant="secondary"
                >
                  삭제
                </Button>
              {/snippet}
            </SettingsRow>
          {/each}
        {/each}
      </SettingsCard>
    {:else}
      <SettingsCard>
        <div class={css({ padding: '20px', fontSize: '13px', color: 'text.subtle', textAlign: 'center' })}>
          에디터에서 업로드한 폰트가 여기 나타나요.
        </div>
      </SettingsCard>
    {/if}
  </div>
</div>
