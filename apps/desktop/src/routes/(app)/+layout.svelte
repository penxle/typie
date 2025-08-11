<script lang="ts">
  import { graphql } from '$graphql';

  const { children } = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query Layout_Query {
      me {
        id
      }
    }
  `);
</script>

{@render children()}
