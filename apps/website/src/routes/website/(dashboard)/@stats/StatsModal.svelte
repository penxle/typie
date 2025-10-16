<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { graphql } from '$graphql';
  import ActivityChart from './ActivityChart.svelte';
  import ActivityGrid from './ActivityGrid.svelte';

  const query = graphql(`
    query DashboardLayout_StatsModal_Query @client {
      me @required {
        id

        ...DashboardLayout_Stats_ActivityChart_user
        ...DashboardLayout_Stats_ActivityGrid_user
      }
    }
  `);

  const generateActivityImage = graphql(`
    mutation DashboardLayout_StatsModal_GenerateActivityImage {
      generateActivityImage
    }
  `);

  const app = getAppContext();
  let loaded = $state(false);

  const load = async () => {
    if (app.state.statsOpen) {
      await query.load();
      loaded = true;
    }
  };

  $effect(() => {
    load();
  });

  const copyActivityImage = async () => {
    const b64 = await generateActivityImage();
    const blob = new Blob([Uint8Array.fromBase64(b64)], { type: 'image/png' });
    await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);

    Toast.success('이미지가 클립보드에 복사되었어요.');
  };
</script>

<Modal
  style={css.raw({
    gap: '16px',
    maxWidth: '800px',
    padding: '24px',
  })}
  loading={!loaded || !query}
  onclose={() => {
    app.state.statsOpen = false;
    loaded = false;
  }}
  open={app.state.statsOpen}
>
  {#if loaded && $query}
    <div class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.subtle' })}>나의 글쓰기 통계</div>

    <div class={flex({ flexDirection: 'column', gap: '32px' })}>
      <div class={flex({ flexDirection: 'column', gap: '16px' })}>
        <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.faint' })}>지난 1년간의 기록</div>

        <ActivityGrid $user={$query.me} />

        <div class={flex({ justifyContent: 'flex-end' })}>
          <Button onclick={copyActivityImage} variant="secondary">이미지로 복사하기</Button>
        </div>
      </div>

      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        <ActivityChart $user={$query.me} />
      </div>
    </div>
  {/if}
</Modal>
