<script lang="ts">
  import { SingleSignOnProvider } from '@/enums';
  import { graphql } from '$graphql';
  import { center } from '$styled-system/patterns';

  const generateSingleSignOnAuthorizationUrl = graphql(`
    mutation LoginPage_GenerateSingleSignOnAuthorizationUrl_Mutation($input: GenerateSingleSignOnAuthorizationUrlInput!) {
      generateSingleSignOnAuthorizationUrl(input: $input)
    }
  `);
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <button
    onclick={async () => {
      const url = await generateSingleSignOnAuthorizationUrl({
        provider: SingleSignOnProvider.GOOGLE,
      });

      location.href = url;
    }}
    type="button"
  >
    구글로 시작하기
  </button>
</div>
