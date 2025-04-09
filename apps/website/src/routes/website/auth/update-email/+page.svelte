<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { toast } from '$lib/notification';
  import { center } from '$styled-system/patterns';

  const updateEmail = graphql(`
    mutation UpdateEmailPage_UpdateEmail_Mutation($input: UpdateEmailInput!) {
      updateEmail(input: $input)
    }
  `);

  onMount(async () => {
    await updateEmail({
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      code: page.url.searchParams.get('code')!,
    });

    toast.success('이메일이 변경되었어요');

    await goto('/', { replaceState: true });
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>처리중...</div>
</div>
