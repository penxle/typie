<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { center } from '$styled-system/patterns';

  const changeEmail = graphql(`
    mutation ChangeEmailPage_ChangeEmail_Mutation($input: ChangeEmailInput!) {
      changeEmail(input: $input)
    }
  `);

  onMount(async () => {
    await changeEmail({
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      code: page.url.searchParams.get('code')!,
    });

    await goto('/', {
      replaceState: true,
    });
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>처리중...</div>
</div>
