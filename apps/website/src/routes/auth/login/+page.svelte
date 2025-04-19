<script lang="ts">
  import qs from 'query-string';
  import { z } from 'zod';
  import { SingleSignOnProvider } from '@/enums';
  import { TypieError } from '@/errors';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { createForm, FormError } from '$lib/form';
  import { serializeOAuthState } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const loginWithEmail = graphql(`
    mutation LoginPage_LoginWithEmail_Mutation($input: LoginWithEmailInput!) {
      loginWithEmail(input: $input)
    }
  `);

  const generateSingleSignOnAuthorizationUrl = graphql(`
    mutation LoginPage_GenerateSingleSignOnAuthorizationUrl_Mutation($input: GenerateSingleSignOnAuthorizationUrlInput!) {
      generateSingleSignOnAuthorizationUrl(input: $input)
    }
  `);

  const form = createForm({
    schema: z.object({
      email: z.string({ required_error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
      password: z.string({ required_error: '비밀번호를 입력해주세요.' }).nonempty('비밀번호를 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      await loginWithEmail({
        email: data.email,
        password: data.password,
      });

      location.href = qs.stringifyUrl({
        url: `${env.PUBLIC_AUTH_URL}/authorize`,
        query: {
          client_id: env.PUBLIC_OIDC_CLIENT_ID,
          response_type: 'code',
          redirect_uri: page.url.searchParams.get('redirect_uri') || `${env.PUBLIC_WEBSITE_URL}/authorize`,
          state: page.url.searchParams.get('state') || serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
        },
      });
    },
    onError: (error) => {
      if (error instanceof TypieError) {
        if (error.code === 'invalid_credentials') {
          throw new FormError('password', '이메일 혹은 비밀번호가 일치하지 않습니다.');
        } else if (error.code === 'password_not_set') {
          throw new FormError('password', '비밀번호가 설정되지 않았습니다.');
        }
      }
    },
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div class={flex({ direction: 'column', gap: { base: '24px' }, maxWidth: '400px', width: 'full', padding: { base: '16px' } })}>
    <h1 class={css({ fontSize: { base: '24px' }, fontWeight: 'bold', textAlign: 'center' })}>로그인</h1>

    <form class={flex({ direction: 'column', gap: { base: '16px' } })} onsubmit={form.handleSubmit}>
      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="email">이메일</label>
        <input
          id="email"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="이메일을 입력하세요"
          type="text"
          bind:value={form.fields.email}
        />

        {#if form.errors.email}
          <p class={css({ color: 'red.500' })}>{form.errors.email}</p>
        {/if}
      </div>

      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="password">비밀번호</label>
        <input
          id="password"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="비밀번호를 입력하세요"
          type="password"
          bind:value={form.fields.password}
        />

        {#if form.errors.password}
          <p class={css({ color: 'red.500' })}>{form.errors.password}</p>
        {/if}
      </div>

      <button
        class={css({ backgroundColor: '[#000000]', color: '[#FFFFFF]', padding: '12px', borderRadius: '4px', fontWeight: '[500]' })}
        disabled={form.state.isLoading}
        type="submit"
      >
        {form.state.isLoading ? '처리 중...' : '로그인'}
      </button>

      <div class={css({ textAlign: 'center', marginTop: '8px' })}>
        <a class={css({ color: 'gray.600', textDecoration: 'underline' })} href={`/forgot-password${page.url.search}`}>
          비밀번호를 잊으셨나요?
        </a>
      </div>
    </form>

    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      <hr class={css({ flex: '1' })} />
      <span class={css({ color: 'gray.500' })}>또는</span>
      <hr class={css({ flex: '1' })} />
    </div>

    <div class={flex({ direction: 'column', gap: { base: '16px' } })}>
      <button
        class={css({
          borderWidth: '1px',
          padding: '12px',
          borderRadius: '4px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '8px',
        })}
        onclick={async () => {
          const url = await generateSingleSignOnAuthorizationUrl({
            provider: SingleSignOnProvider.GOOGLE,
            state: serializeOAuthState({
              redirect_uri: page.url.searchParams.get('redirect_uri') || `${env.PUBLIC_WEBSITE_URL}/authorize`,
              state: page.url.searchParams.get('state') || serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
            }),
          });

          location.href = url;
        }}
        type="button"
      >
        구글로 시작하기
      </button>

      <button
        class={css({
          borderWidth: '1px',
          padding: '12px',
          borderRadius: '4px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '8px',
        })}
        onclick={async () => {
          const url = await generateSingleSignOnAuthorizationUrl({
            provider: SingleSignOnProvider.KAKAO,
            state: serializeOAuthState({
              redirect_uri: page.url.searchParams.get('redirect_uri') || `${env.PUBLIC_WEBSITE_URL}/authorize`,
              state: page.url.searchParams.get('state') || serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
            }),
          });

          location.href = url;
        }}
        type="button"
      >
        카카오로 시작하기
      </button>

      <button
        class={css({
          borderWidth: '1px',
          padding: '12px',
          borderRadius: '4px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '8px',
        })}
        onclick={async () => {
          const url = await generateSingleSignOnAuthorizationUrl({
            provider: SingleSignOnProvider.NAVER,
            state: serializeOAuthState({
              redirect_uri: page.url.searchParams.get('redirect_uri') || `${env.PUBLIC_WEBSITE_URL}/authorize`,
              state: page.url.searchParams.get('state') || serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
            }),
          });

          location.href = url;
        }}
        type="button"
      >
        네이버로 시작하기
      </button>

      <div class={css({ textAlign: 'center', marginTop: '16px' })}>
        <p>
          계정이 없으신가요? <a class={css({ textDecoration: 'underline' })} href={`/signup${page.url.search}`}>이메일로 회원가입</a>
        </p>
      </div>
    </div>
  </div>
</div>
