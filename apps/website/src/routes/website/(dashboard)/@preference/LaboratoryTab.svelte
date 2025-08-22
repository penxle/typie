<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
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

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>실험실</h1>

  <div class={css({ fontSize: '14px', color: 'text.muted' })}>
    <p>실험실 기능은 아직 개발 중이거나 테스트 중인 기능들입니다.</p>
    <p class={css({ marginTop: '8px' })}>이 기능들은 언제든지 변경되거나 제거될 수 있습니다.</p>
  </div>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>PDF 내보내기</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>PDF 내보내기 기능을 활성화합니다.</p>
    </div>

    <Switch
      onchange={() => {
        mixpanel.track('toggle_experimental_feature', {
          feature: 'pdf_export',
          enabled: app.preference.current.experimental_pdfExportEnabled,
        });
      }}
      bind:checked={app.preference.current.experimental_pdfExportEnabled}
    />
  </div>
</div>
