<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_LaboratoryTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_LaboratoryTab_user;
  };

  let { $user: _user }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_LaboratoryTab_user on User {
        id
      }
    `),
  );

  const app = getAppContext();
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '12px' })}>실험실</h1>

    <div class={css({ fontSize: '14px', color: 'text.muted' })}>
      <p>실험실 기능은 아직 개발 중이거나 테스트 중인 기능들이에요.</p>
      <p class={css({ marginTop: '4px' })}>이 기능들은 언제든지 변경되거나 제거될 수 있어요.</p>
    </div>
  </div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        PDF 내보내기
      {/snippet}
      {#snippet description()}
        문서를 PDF 파일로 내보낼 수 있어요.
      {/snippet}
      {#snippet value()}
        <Switch
          onchange={() => {
            mixpanel.track('toggle_experimental_feature', {
              feature: 'pdf_export',
              enabled: app.preference.current.experimental_pdfExportEnabled,
            });
          }}
          bind:checked={app.preference.current.experimental_pdfExportEnabled}
        />
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow>
      {#snippet label()}
        DOCX 내보내기
      {/snippet}
      {#snippet description()}
        문서를 워드(DOCX) 파일로 내보낼 수 있어요.
      {/snippet}
      {#snippet value()}
        <Switch
          onchange={() => {
            mixpanel.track('toggle_experimental_feature', {
              feature: 'docx_export',
              enabled: app.preference.current.experimental_docxExportEnabled,
            });
          }}
          bind:checked={app.preference.current.experimental_docxExportEnabled}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
