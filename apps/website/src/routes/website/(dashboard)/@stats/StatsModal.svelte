<script lang="ts">
  import { graphql } from '$graphql';
  import { Modal } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import ActivityGrid from './ActivityGrid.svelte';

  const query = graphql(`
    query DashboardLayout_StatsModal_Query @client {
      me @required {
        id

        ...DashboardLayout_Stats_ActivityGrid_user
      }
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
    <div class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.700' })}>나의 글쓰기 통계</div>

    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'gray.500' })}>지난 1년간의 기록</div>

      <ActivityGrid $user={$query.me} />
    </div>
  {/if}
</Modal>
