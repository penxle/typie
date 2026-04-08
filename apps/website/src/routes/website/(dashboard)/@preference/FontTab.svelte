<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import PlusIcon from '~icons/lucide/plus';
  import { FontSpecimen, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { familySpecimenFallbacks, weightSpecimenFallbacks } from '$lib/components/font-specimen';
  import { getRepresentativeFont } from '$lib/editor/fonts';
  import { values } from '$lib/editor/values';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import FontUploadModal from '../FontUploadModal.svelte';
  import { PlanUpgradeDialog } from '../plan-upgrade-dialog.svelte';
  import type { DashboardLayout_PreferenceModal_FontTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_FontTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_FontTab_user on User {
        id
        documentFontFamilies {
          id
          familyName
          displayName
          source
          state

          fonts {
            id
            weight
            state
            subfamilyDisplayName
            url
          }
        }

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const documentFontFamilies = $derived(user.data?.documentFontFamilies ?? []);

  const userFontFamilies = $derived(
    documentFontFamilies
      .filter((f) => f.source === 'USER' && f.state === 'ACTIVE')
      .map((family) => ({
        ...family,
        fonts: [
          ...new Map(
            family.fonts
              .filter((f) => f.state === 'ACTIVE')
              .toSorted((a, b) => a.weight - b.weight)
              .map((f) => [f.weight, f]),
          ).values(),
        ],
      }))
      .filter((family) => family.fonts.length > 0),
  );

  let uploadModalOpen = $state(false);

  const [archiveFontFamily] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_FontTab_ArchiveFontFamily_Mutation($input: ArchiveFontFamilyInput!) {
        archiveFontFamily(input: $input) {
          id
        }
      }
    `),
  );

  const [archiveFont] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_FontTab_ArchiveFont_Mutation($input: ArchiveFontInput!) {
        archiveFont(input: $input) {
          id
        }
      }
    `),
  );

  const getWeightLabel = (font: { weight: number; subfamilyDisplayName?: string | null }) => {
    return (
      values.fontWeight.find((f) => f.value === font.weight)?.label ??
      (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight))
    );
  };
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
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
        if (user.data?.subscription) {
          uploadModalOpen = true;
        } else {
          PlanUpgradeDialog.show({
            message: '폰트 업로드 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.',
          });
        }
      }}
      type="button"
    >
      <Icon style={css.raw({ color: 'text.faint' })} icon={PlusIcon} size={14} />
      <span>직접 업로드</span>
    </button>
  </div>

  {#if userFontFamilies.length > 0}
    {#each userFontFamilies as family (family.id)}
      <div>
        <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '20px' })}>
          <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>
            <FontSpecimen
              fallbacks={familySpecimenFallbacks(family.displayName, family.familyName)}
              fontId={getRepresentativeFont(family.fonts)?.id}
              text={family.displayName}
              weight={getRepresentativeFont(family.fonts)?.weight}
            />
          </h2>
          <button
            class={css({
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
              Dialog.confirm({
                title: '폰트 패밀리 삭제',
                message: `"${family.displayName}" 폰트 패밀리 전체를 삭제하시겠어요?`,
                action: 'danger',
                actionLabel: '삭제',
                actionHandler: async () => {
                  await archiveFontFamily({ input: { fontFamilyId: family.id } });
                  cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'fontFamilies' });
                  cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'documentFontFamilies' });
                  cache.invalidate({ __typename: 'Document', $field: 'fontFamilies' });
                },
              });
            }}
            type="button"
          >
            전체 삭제
          </button>
        </div>

        <SettingsCard>
          {#each family.fonts as font, fontIndex (font.id)}
            {#if fontIndex > 0}
              <SettingsDivider />
            {/if}
            <SettingsRow>
              {#snippet label()}
                <FontSpecimen
                  fallbacks={weightSpecimenFallbacks(getWeightLabel(font), font.subfamilyDisplayName, font.weight)}
                  fontId={font.id}
                  text={getWeightLabel(font)}
                  weight={font.weight}
                />
              {/snippet}
              {#snippet value()}
                <Button
                  onclick={() => {
                    Dialog.confirm({
                      title: '폰트 삭제',
                      message: `"${family.displayName} ${getWeightLabel(font)}" 폰트를 삭제하시겠어요?`,
                      action: 'danger',
                      actionLabel: '삭제',
                      actionHandler: async () => {
                        await archiveFont({ input: { fontId: font.id } });
                        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'documentFontFamilies' });
                        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'fontFamilies' });
                        cache.invalidate({ __typename: 'Document', $field: 'fontFamilies' });
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
        </SettingsCard>
      </div>
    {/each}
  {:else}
    <SettingsCard>
      <div class={css({ paddingX: '20px', paddingY: '40px', fontSize: '13px', color: 'text.subtle', textAlign: 'center' })}>
        아직 직접 업로드한 폰트가 없어요.
        <br />
        우측 상단의 직접 업로드 버튼이나 문서 에디터의 폰트 패밀리 메뉴에서 추가할 수 있어요.
      </div>
    </SettingsCard>
  {/if}
</div>

{#if user.data}
  <FontUploadModal userId={user.data.id} bind:open={uploadModalOpen} />
{/if}
