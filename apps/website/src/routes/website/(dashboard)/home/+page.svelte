<script lang="ts">
  import { graphql } from '$graphql';
  import { center } from '$styled-system/patterns';
  import TopBar from '../TopBar.svelte';

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query HomePage_Query {
      me @required {
        id
      }
    }
  `);
</script>

<TopBar />

<div class={center({ flexDirection: 'column', flexGrow: '1', width: 'full' })}>Home</div>
