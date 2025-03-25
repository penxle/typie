<script lang="ts">
  import { SingleSignOnProvider } from '@/enums';
  import { graphql } from '$graphql';
  import { center, flex } from '$styled-system/patterns';

  const generateSingleSignOnAuthorizationUrl = graphql(`
    mutation LoginPage_GenerateSingleSignOnAuthorizationUrl_Mutation($input: GenerateSingleSignOnAuthorizationUrlInput!) {
      generateSingleSignOnAuthorizationUrl(input: $input)
    }
  `);
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div class={flex({ direction: 'column', gap: '4px' })}>
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

    <button
      onclick={async () => {
        const url = await generateSingleSignOnAuthorizationUrl({
          provider: SingleSignOnProvider.KAKAO,
        });

        location.href = url;
      }}
      type="button"
    >
      카카오로 시작하기
    </button>

    <button
      onclick={async () => {
        const url = await generateSingleSignOnAuthorizationUrl({
          provider: SingleSignOnProvider.NAVER,
        });

        location.href = url;
      }}
      type="button"
    >
      네이버로 시작하기
    </button>
  </div>
</div>
