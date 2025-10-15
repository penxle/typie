<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import PlusIcon from '~icons/lucide/plus';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import FontUploadModal from '../FontUploadModal.svelte';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
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

        subscription {
          id
        }
      }
    `),
  );

  let uploadModalOpen = $state(false);
  let planUpgradeOpen = $state(false);

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
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>폰트</h1>
    <button
      class={flex({
        alignItems: 'center',
        gap: '6px',
        borderRadius: '6px',
        paddingX: '12px',
        paddingY: '6px',
        fontSize: '13px',
        fontWeight: 'medium',
        color: 'text.subtle',
        transition: 'common',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      onclick={() => {
        if ($user.subscription) {
          uploadModalOpen = true;
        } else {
          planUpgradeOpen = true;
        }
      }}
      type="button"
    >
      <Icon style={css.raw({ color: 'text.faint' })} icon={PlusIcon} size={14} />
      <span>직접 업로드</span>
    </button>
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

<FontUploadModal userId={$user.id} bind:open={uploadModalOpen} />
<PlanUpgradeModal bind:open={planUpgradeOpen}>폰트 업로드 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.</PlanUpgradeModal>
