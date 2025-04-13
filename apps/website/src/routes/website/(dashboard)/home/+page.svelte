<script lang="ts">
  import { graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import TopBar from '../TopBar.svelte';
  import ActivityGrid from './ActivityGrid.svelte';

  const query = graphql(`
    query HomePage_Query {
      me @required {
        id

        ...HomePage_ActivityGrid_user
      }
    }
  `);
</script>

<TopBar />

<div class={center({ flexDirection: 'column', flexGrow: '1', width: 'full' })}>
  <div class={flex({ flexDirection: 'column', flexGrow: '1', gap: '8px', width: 'full', maxWidth: '1000px' })}>
    <div class={css({ fontSize: '14px', fontWeight: 'semibold' })}>나의 기록</div>

    <ActivityGrid $user={$query.me} />
  </div>
</div>
