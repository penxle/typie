<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { z } from 'zod';
  import { SingleSignOnProvider } from '@/enums';
  import { TypieError } from '@/errors';
  import NaverIcon from '~icons/simple-icons/naver';
  import GoogleIcon from '~icons/typie/google';
  import KakaoIcon from '~icons/typie/kakao';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { fb } from '$lib/analytics';
  import { Button, Helmet, Icon, TextInput } from '$lib/components';
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
      email: z.string({ error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
      password: z.string({ error: '비밀번호를 입력해주세요.' }).min(1, '비밀번호를 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      await loginWithEmail({
        email: data.email,
        password: data.password,
      });

      mixpanel.track('login_with_email');

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
          throw new FormError('password', '이메일 또는 비밀번호가 올바르지 않아요.');
        } else if (error.code === 'password_not_set') {
          throw new FormError('password', '비밀번호가 설정되지 않았어요.');
        }
      }
    },
  });

  $effect(() => {
    void form;
  });

  const singleSignOn = async (provider: SingleSignOnProvider) => {
    const url = await generateSingleSignOnAuthorizationUrl({
      provider,
      state: serializeOAuthState({
        redirect_uri: page.url.searchParams.get('redirect_uri') || `${env.PUBLIC_WEBSITE_URL}/authorize`,
        state: page.url.searchParams.get('state') || serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
      }),
    });

    mixpanel.track('login_with_sso', { provider });
    fb.track('CompleteRegistration');

    location.href = url;
  };
</script>

<Helmet description="지금 타이피에 로그인하고 바로 글쓰기를 시작해보세요." title="로그인" />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>타이피에 오신 것을 환영해요!</h1>

    <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint' })}>
      아직 계정이 없으신가요?
      <a
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          fontWeight: 'medium',
          color: 'text.default',
          _hover: { textDecoration: 'underline', textUnderlineOffset: '2px' },
        })}
        href={`/signup${page.url.search}`}
      >
        이메일로 회원가입하기
      </a>
    </div>
  </div>

  <form class={flex({ flexDirection: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        <label class={css({ fontSize: '13px', color: 'text.subtle', userSelect: 'none' })} for="email">이메일</label>

        <TextInput id="email" aria-invalid={!!form.errors.email} placeholder="me@example.com" bind:value={form.fields.email} />

        {#if form.errors.email}
          <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.email}</div>
        {/if}
      </div>

      <div class={flex({ direction: 'column', gap: '4px' })}>
        <label class={css({ fontSize: '13px', color: 'text.subtle', userSelect: 'none' })} for="password">비밀번호</label>

        <TextInput
          id="password"
          aria-invalid={!!form.errors.password}
          placeholder="********"
          type="password"
          bind:value={form.fields.password}
        />

        {#if form.errors.password}
          <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.password}</div>
        {/if}
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <Button style={css.raw({ height: '40px' })} loading={form.state.isLoading} size="lg" type="submit">로그인</Button>

      <div class={center()}>
        <a
          class={css({ fontSize: '13px', color: 'text.subtle', _hover: { textDecoration: 'underline', textUnderlineOffset: '2px' } })}
          href={`/forgot-password${page.url.search}`}
        >
          비밀번호를 잊으셨나요?
        </a>
      </div>
    </div>
  </form>

  <div class={flex({ alignItems: 'center', gap: '16px', userSelect: 'none' })}>
    <div class={css({ flex: '1', height: '1px', backgroundColor: 'interactive.hover' })}></div>
    <span class={css({ fontSize: '14px', color: 'text.faint' })}>간편 로그인</span>
    <div class={css({ flex: '1', height: '1px', backgroundColor: 'interactive.hover' })}></div>
  </div>

  <div class={flex({ justifyContent: 'space-between', gap: '16px', height: { base: '36px', lg: '40px' } })}>
    <button
      class={center({
        flex: '1',
        borderWidth: '1px',
        borderRadius: '8px',
        backgroundColor: 'surface.default',
      })}
      onclick={() => singleSignOn(SingleSignOnProvider.GOOGLE)}
      type="button"
    >
      <Icon icon={GoogleIcon} />
    </button>

    <button
      class={center({
        flex: '1',
        borderWidth: '1px',
        borderColor: '[#FEE500]',
        borderRadius: '8px',
        color: '[#000000]',
        backgroundColor: '[#FEE500]',
      })}
      onclick={() => singleSignOn(SingleSignOnProvider.KAKAO)}
      type="button"
    >
      <Icon icon={KakaoIcon} />
    </button>

    <button
      class={center({
        flex: '1',
        borderWidth: '1px',
        borderColor: '[#03C75A]',
        borderRadius: '8px',
        color: 'text.bright',
        backgroundColor: '[#03C75A]',
      })}
      onclick={() => singleSignOn(SingleSignOnProvider.NAVER)}
      type="button"
    >
      <Icon icon={NaverIcon} />
    </button>
  </div>
</div>
