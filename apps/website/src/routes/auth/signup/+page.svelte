<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import ShuffleIcon from '~icons/lucide/dices';
  import NaverIcon from '~icons/simple-icons/naver';
  import GoogleIcon from '~icons/typie/google';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Button, Checkbox, Helmet, Icon, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { serializeOAuthState } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const query = graphql(`
    query SignUpPage_Query {
      randomName
    }
  `);

  const sendSignUpEmail = graphql(`
    mutation SignUpPage_SendSignUpEmail_Mutation($input: SendSignUpEmailInput!) {
      sendSignUpEmail(input: $input)
    }
  `);

  const generateRandomName = graphql(`
    mutation SignUpPage_GenerateRandomName_Mutation {
      generateRandomName
    }
  `);

  const form = createForm({
    schema: z
      .object({
        name: z.string().optional(),
        email: z.string({ error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
        password: z.string({ error: '비밀번호를 입력해주세요.' }).min(1, '비밀번호를 입력해주세요.'),
        confirmPassword: z.string({ error: '비밀번호 확인을 입력해주세요.' }).min(1, '비밀번호 확인을 입력해주세요.'),
        termsAgreed: z.boolean().refine((val) => val === true, {
          message: '이용약관 및 개인정보처리방침에 동의해주세요.',
        }),
        marketingAgreed: z.boolean().optional(),
      })
      .refine((data) => data.password === data.confirmPassword, {
        path: ['confirmPassword'],
        message: '비밀번호가 일치하지 않습니다.',
      }),
    onSubmit: async (data) => {
      await sendSignUpEmail({
        email: data.email,
        name: data.name ?? name,
        password: data.password,
        state: serializeOAuthState({
          redirect_uri: page.url.searchParams.get('redirect_uri') || `${env.PUBLIC_WEBSITE_URL}/authorize`,
          state: page.url.searchParams.get('state') || serializeOAuthState({ redirect_uri: env.PUBLIC_WEBSITE_URL }),
        }),
        marketingAgreed: data.marketingAgreed ?? false,
      });

      mixpanel.track('send_sign_up_email');

      emailSent = true;
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'user_email_exists') {
        throw new FormError('email', '이미 사용중인 이메일입니다.');
      }
    },
    defaultValues: {
      termsAgreed: false,
      marketingAgreed: false,
    },
  });

  $effect(() => {
    void form;
  });

  let name = $state($query.randomName);
  let emailSent = $state(false);
</script>

<Helmet
  description="지금 타이피에 가입하고 바로 글쓰기를 시작해보세요."
  image={{ size: 'large', src: 'https://cdn.typie.net/opengraph/default.png' }}
  title="회원가입"
/>

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  {#if !emailSent}
    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>지금 타이피에 가입하세요</h1>

      <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint' })}>
        이미 계정이 있으신가요?
        <a
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            fontWeight: 'medium',
            color: 'text.default',
            _hover: { textDecoration: 'underline', textUnderlineOffset: '2px' },
          })}
          href={`/login${page.url.search}`}
        >
          로그인하기
        </a>
      </div>
    </div>

    <form class={flex({ flexDirection: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
      <div class={flex({ direction: 'column', gap: '12px' })}>
        <div class={flex({ direction: 'column', gap: '4px' })}>
          <label class={css({ fontSize: '13px', color: 'text.subtle', userSelect: 'none' })} for="email">이메일</label>

          <TextInput id="email" aria-invalid={!!form.errors.email} autofocus placeholder="me@example.com" bind:value={form.fields.email} />

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

        <div class={flex({ direction: 'column', gap: '4px' })}>
          <label class={css({ fontSize: '13px', color: 'text.subtle', userSelect: 'none' })} for="confirmPassword">비밀번호 확인</label>

          <TextInput
            id="confirmPassword"
            aria-invalid={!!form.errors.confirmPassword}
            placeholder="********"
            type="password"
            bind:value={form.fields.confirmPassword}
          />

          {#if form.errors.confirmPassword}
            <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.confirmPassword}</div>
          {/if}
        </div>

        <div class={flex({ direction: 'column', gap: '4px' })}>
          <label class={css({ fontSize: '13px', color: 'text.subtle', userSelect: 'none' })} for="name">닉네임</label>

          <TextInput id="name" aria-invalid={!!form.errors.name} placeholder={name} bind:value={form.fields.name}>
            {#snippet rightItem()}
              <button
                class={center({
                  borderRadius: '6px',
                  size: '24px',
                  color: 'text.faint',
                  _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
                })}
                onclick={async () => {
                  name = await generateRandomName();
                }}
                type="button"
                use:tooltip={{ message: '주사위 굴리기', placement: 'top', offset: 8, keepOnClick: true }}
              >
                <Icon icon={ShuffleIcon} size={14} />
              </button>
            {/snippet}
          </TextInput>

          {#if form.errors.name}
            <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.name}</div>
          {/if}
        </div>

        <div class={flex({ direction: 'column', gap: '6px', marginTop: '8px' })}>
          <div class={flex({ direction: 'column', gap: '4px' })}>
            <Checkbox
              id="termsAgreed"
              name="termsAgreed"
              aria-invalid={!!form.errors.termsAgreed}
              size="sm"
              bind:checked={form.fields.termsAgreed}
            >
              <span class={flex({ wrap: 'wrap', fontSize: { base: '13px', lg: '14px' }, color: 'text.subtle' })}>
                <a
                  class={css({ textDecoration: 'underline', color: 'text.default' })}
                  href="https://help.typie.co/legal/terms"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  이용약관
                </a>

                <span>&nbsp;및&nbsp;</span>

                <a
                  class={css({ textDecoration: 'underline', color: 'text.default' })}
                  href="https://help.typie.co/legal/privacy"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  개인정보처리방침
                </a>

                <span>에&nbsp;</span>
                <span>동의해요&nbsp;</span>
                <span>(필수)</span>
              </span>
            </Checkbox>

            {#if form.errors.termsAgreed}
              <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.termsAgreed}</div>
            {/if}
          </div>

          <Checkbox id="marketingAgreed" name="marketingAgreed" size="sm" bind:checked={form.fields.marketingAgreed}>
            <span class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.subtle' })}>마케팅 정보 수신에 동의해요 (선택)</span>
          </Checkbox>
        </div>
      </div>

      <Button style={css.raw({ height: '40px' })} loading={form.state.isLoading} size="lg" type="submit">가입하기</Button>
    </form>
  {:else}
    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>지금 타이피에 가입하세요</h1>

      <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint', wordBreak: 'keep-all' })}>
        {form.fields.email} 으로 회원가입 링크를 보냈어요.
      </div>
    </div>

    <div class={flex({ direction: 'column', gap: '4px' })}>
      <Button style={center.raw({ gap: '8px', width: 'full' })} external href="https://gmail.com" size="lg" type="link" variant="secondary">
        <Icon icon={GoogleIcon} size={16} />
        구글 이메일 열기
      </Button>

      <Button
        style={center.raw({ gap: '8px', width: 'full' })}
        external
        href="https://mail.naver.com"
        size="lg"
        type="link"
        variant="secondary"
      >
        <Icon style={css.raw({ color: '[#03C75A]' })} icon={NaverIcon} size={14} />
        네이버 이메일 열기
      </Button>
    </div>

    <div class={flex({ justifyContent: 'center' })}>
      <a
        class={css({ fontSize: '13px', color: 'text.subtle', _hover: { textDecoration: 'underline', textUnderlineOffset: '2px' } })}
        href={`/login${page.url.search}`}
      >
        로그인 페이지로 돌아가기
      </a>
    </div>
  {/if}
</div>
