<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { center } from '$styled-system/patterns';

  const authorizeSignUpEmail = graphql(`
    mutation EmailPage_AuthorizeSignUpEmail_Mutation($input: AuthorizeSignUpEmailInput!) {
      authorizeSignUpEmail(input: $input) {
        id
      }
    }
  `);

  onMount(async () => {
    await authorizeSignUpEmail({
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      code: page.url.searchParams.get('code')!,
    });

    await goto('/', {
      replaceState: true,
    });
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>logging in...</div>
</div>
