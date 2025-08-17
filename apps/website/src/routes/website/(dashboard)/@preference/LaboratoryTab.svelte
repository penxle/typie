<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select, Switch } from '@typie/ui/components';
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

  const pageLayouts = [
    {
      label: 'A4 (210mm x 297mm)',
      description: '세로 210mm, 가로 297mm, 여백 25mm',
      value: 'a4',
    },
    {
      label: 'A5 (148mm x 210mm)',
      description: '세로 148mm, 가로 210mm, 여백 20mm',
      value: 'a5',
    },
    {
      label: 'B5 (176mm x 250mm)',
      description: '세로 176mm, 가로 250mm, 여백 15mm',
      value: 'b5',
    },
    {
      label: 'B6 (125mm x 176mm)',
      description: '세로 125mm, 가로 176mm, 여백 10mm',
      value: 'b6',
    },
  ];
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>실험실</h1>

  <div class={css({ fontSize: '14px', color: 'text.muted' })}>
    <p>실험실 기능은 아직 개발 중이거나 테스트 중인 기능들입니다.</p>
    <p class={css({ marginTop: '8px' })}>이 기능들은 언제든지 변경되거나 제거될 수 있습니다.</p>
  </div>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>페이지 보기</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>에디터에서 페이지 보기를 활성화합니다.</p>
    </div>

    <Switch
      onchange={() => {
        mixpanel.track('toggle_experimental_feature', {
          feature: 'page_view',
          enabled: app.preference.current.experimental_pageEnabled,
        });

        if (app.preference.current.experimental_pageEnabled && !app.preference.current.experimental_pageLayoutId) {
          app.preference.current.experimental_pageLayoutId = pageLayouts[0].value;
        }
      }}
      bind:checked={app.preference.current.experimental_pageEnabled}
    />
  </div>

  {#if app.preference.current.experimental_pageEnabled}
    <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
      <div>
        <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>페이지 레이아웃</h3>
        <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>페이지 크기와 여백을 설정합니다.</p>
      </div>

      <Select items={pageLayouts} bind:value={app.preference.current.experimental_pageLayoutId} />
    </div>
  {/if}

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
