<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Checkbox, ContentProtect, Helmet, HorizontalDivider, Icon, Modal, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { setupEditorContext, TiptapRenderer } from '@typie/ui/tiptap';
  import { comma, serializeOAuthState } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import qs from 'query-string';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { cardSchema } from '@/validation';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import LockIcon from '~icons/lucide/lock';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import SmileIcon from '~icons/lucide/smile';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { Img } from '$lib/components';
  import ContentNavigation from './ContentNavigation.svelte';
  import EmojiReaction from './EmojiReaction.svelte';
  import PostActionMenu from './PostActionMenu.svelte';
  import PostViewBodyUnavailable from './PostViewBodyUnavailable.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { Optional, UsersiteWildcardSlugPage_PostView_entityView, UsersiteWildcardSlugPage_PostView_user } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_PostView_entityView;
    $user: Optional<UsersiteWildcardSlugPage_PostView_user>;
  };

  let { $entityView: _entityView, $user: _user }: Props = $props();

  const entityView = fragment(
    _entityView,
    graphql(`
      fragment UsersiteWildcardSlugPage_PostView_entityView on EntityView {
        id
        slug
        url

        ancestors {
          id
          slug

          node {
            __typename

            ... on FolderView {
              id
              name
            }
          }
        }

        node {
          __typename

          ... on PostView {
            id
            title
            subtitle
            excerpt
            maxWidth
            protectContent
            allowReaction

            coverImage {
              id
              ...Img_image
            }

            body {
              __typename

              ... on PostViewBodyAvailable {
                content
              }

              ... on PostViewBodyUnavailable {
                reason
              }
            }

            reactions {
              id
              emoji
            }

            ...UsersiteWildcardSlugPage_EmojiReaction_postView
          }
        }

        site {
          id
          name
          url

          logo {
            id
            ...Img_image
          }

          fonts {
            id
            weight
            url

            family {
              id
            }
          }
        }

        ...UsersiteWildcardSlugPage_PostActionMenu_entityView
        ...UsersiteWildcardSlugPage_ContentNavigation_entityView
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment UsersiteWildcardSlugPage_PostView_user on User {
        id

        billingKey {
          id
        }
      }
    `),
  );

  const unlockPostView = graphql(`
    mutation UsersiteWildcardSlugPage_UnlockPostView_Mutation($input: UnlockPostViewInput!) {
      unlockPostView(input: $input) {
        id

        body {
          __typename

          ... on PostViewBodyAvailable {
            content
          }

          ... on PostViewBodyUnavailable {
            reason
          }
        }
      }
    }
  `);

  const verifyPersonalIdentity = graphql(`
    mutation UsersiteWildcardSlugPage_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
      verifyPersonalIdentity(input: $input) {
        id

        personalIdentity {
          id
          expiresAt
        }
      }
    }
  `);

  const updateBillingKey = graphql(`
    mutation UsersiteWildcardSlugPage_UpdateBillingKey_Mutation($input: UpdateBillingKeyInput!) {
      updateBillingKey(input: $input) {
        id
        name
      }
    }
  `);

  const purchasePaywall = graphql(`
    mutation UsersiteWildcardSlugPage_PurchasePaywall_Mutation($input: PurchasePaywallInput!) {
      purchasePaywall(input: $input)
    }
  `);

  const form = createForm({
    schema: z.object({
      password: z.string(),
    }),
    onSubmit: async (data) => {
      if ($entityView.node.__typename !== 'PostView') {
        return;
      }

      await unlockPostView({
        postId: $entityView.node.id,
        password: data.password,
      });

      mixpanel.track('unlock_post_view', {
        postId: $entityView.node.id,
      });
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'invalid_password') {
        throw new FormError('password', '비밀번호가 올바르지 않습니다.');
      }
    },
  });

  $effect(() => {
    void form;
  });

  let paywallNodeId = $state<string | null>(null);
  let paywallPrice = $state<number | null>(null);
  let paywallModalOpen = $state(false);
  let showCardRegistration = $state(false);
  let cardSubmitError = $state<string | null>(null);
  let hasBillingKey = $state($user?.billingKey !== null);

  const cardAgreements = [
    { name: '타이피 결제 이용약관', url: 'https://typie.co/legal/terms' },
    { name: 'NICEPAY 전자금융거래 기본약관', url: 'https://www.nicepay.co.kr/cs/terms/policy1.do' },
  ];

  let cardAgreementChecks = $state(cardAgreements.map(() => false));
  const allCardAgreementsChecked = $derived(cardAgreementChecks.every(Boolean));

  const handleAllCardAgreementCheck = () => {
    cardAgreementChecks = cardAgreementChecks.map(() => !allCardAgreementsChecked);
  };

  const cardForm = createForm({
    schema: z.object({
      cardNumber: cardSchema.cardNumber,
      expiryDate: cardSchema.expiryDate,
      birthOrBusinessRegistrationNumber: cardSchema.birthOrBusinessRegistrationNumber,
      passwordTwoDigits: cardSchema.passwordTwoDigits,
      agreementsAccepted: z.boolean(),
    }),
    defaultValues: {
      agreementsAccepted: false,
    },
    onSubmit: async (data) => {
      cardSubmitError = null;

      if (!data.agreementsAccepted) {
        throw new FormError('agreementsAccepted', '약관에 동의해주세요.');
      }

      await updateBillingKey({
        cardNumber: data.cardNumber,
        expiryDate: data.expiryDate,
        birthOrBusinessRegistrationNumber: data.birthOrBusinessRegistrationNumber,
        passwordTwoDigits: data.passwordTwoDigits,
      });

      mixpanel.track('paywall_register_card');
      hasBillingKey = true;
      showCardRegistration = false;
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'billing_key_issue_failed') {
        cardSubmitError = '카드 정보를 확인해주세요.';
      }
    },
  });

  $effect(() => {
    void cardForm;
  });

  $effect(() => {
    cardForm.fields.agreementsAccepted = allCardAgreementsChecked;
  });

  const formatCardNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    const parts = [value.slice(0, 4), value.slice(4, 8), value.slice(8, 12), value.slice(12)];
    input.value = parts.filter(Boolean).join('-');
  };

  const formatCardExpiry = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    input.value = value.length > 2 ? value.slice(0, 2) + '/' + value.slice(2, 4) : value;
  };

  const formatBusinessNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');

    if (value.length <= 6) {
      input.value = value;
    } else {
      const parts = [value.slice(0, 3), value.slice(3, 5), value.slice(5)];
      input.value = parts.filter(Boolean).join('-');
    }
  };

  const handlePaywallPurchase = (nodeId: string, price: number) => {
    if (!$user) {
      window.location.href = authorizeUrl;
      return;
    }

    paywallNodeId = nodeId;
    paywallPrice = price;
    paywallModalOpen = true;
    showCardRegistration = false;
    cardForm.reset();
    cardSubmitError = null;
  };

  setupEditorContext({
    onPaywallPurchase: handlePaywallPurchase,
  });

  const fontFaces = $derived(
    $entityView.site.fonts
      .flatMap((font) => [
        `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
        `@font-face { font-family: ${font.family.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      ])
      .join('\n'),
  );

  const authorizeUrl = $derived(
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/authorize`,
      query: {
        client_id: env.PUBLIC_OIDC_CLIENT_ID,
        response_type: 'code',
        redirect_uri: `${page.url.origin}/authorize`,
        state: serializeOAuthState({ redirect_uri: page.url.href }),
      },
    }),
  );

  const handleVerification = async () => {
    try {
      mixpanel.track('verify_personal_identity_start');
      sessionStorage.setItem('redirect_uri', page.url.href);

      const resp = await PortOne.requestIdentityVerification({
        storeId: 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1',
        identityVerificationId: `identity-verification-${nanoid()}`,
        channelKey: 'channel-key-31e03361-26cb-4810-86ed-801cce4f570f',
        redirectUrl: `${page.url.origin}/identity`,
      });

      if (resp === undefined) {
        return;
      }

      await verifyPersonalIdentity({
        identityVerificationId: resp.identityVerificationId,
      });

      mixpanel.track('verify_personal_identity_success');
      location.reload();
    } catch (err) {
      const errorMessages: Record<string, string> = {
        identity_verification_failed: '인증에 실패했습니다.',
        same_identity_exists: '이미 다른 계정에 인증된 정보입니다.',
      };

      if (err instanceof TypieError) {
        const message = errorMessages[err.code] || err.code;
        Toast.error(message);
      }
    }
  };
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />

  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

{#if $entityView.node.__typename === 'PostView'}
  <Helmet
    description={$entityView.node.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$entityView.id}` }}
    title={$entityView.node.title}
  />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full' })}>
    <div
      style:--prosemirror-max-width={`${$entityView.node.maxWidth}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingX: '20px',
        paddingBottom: { base: '60px', lg: '80px' },
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'surface.default',
      })}
    >
      {#if $entityView.node.coverImage}
        <div class={css({ width: 'full', marginBottom: '40px' })}>
          <Img
            style={css.raw({ width: 'full' })}
            $image={$entityView.node.coverImage}
            alt="커버 이미지"
            progressive
            ratio={5 / 2}
            size="full"
          />
        </div>
      {/if}

      <div
        class={css({
          paddingTop: $entityView.node.coverImage ? '0' : { base: '48px', md: '80px' },
          width: 'full',
          maxWidth: 'var(--prosemirror-max-width)',
        })}
      >
        <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
          <nav class={flex({ alignItems: 'center', gap: '6px', flexWrap: 'wrap', marginBottom: '20px' })}>
            <a class={flex({ alignItems: 'center', gap: '6px' })} href={$entityView.site.url}>
              {#if $entityView.site.logo}
                <Img
                  style={css.raw({ size: '18px', borderRadius: '4px', objectFit: 'cover' })}
                  $image={$entityView.site.logo}
                  alt={`${$entityView.site.name} 로고`}
                  size={24}
                />
              {/if}
              <span class={css({ fontSize: '13px', color: 'text.faint', _hover: { color: 'text.muted' } })}>
                {$entityView.site.name}
              </span>
            </a>

            {#each $entityView.ancestors as ancestor (ancestor.id)}
              {#if ancestor.node.__typename === 'FolderView'}
                <span class={css({ fontSize: '13px', color: 'text.faint' })}>/</span>
                <a class={css({ fontSize: '13px', color: 'text.faint', _hover: { color: 'text.muted' } })} href={`/${ancestor.slug}`}>
                  {ancestor.node.name}
                </a>
              {/if}
            {/each}
          </nav>

          <h1 class={css({ fontSize: '22px', fontWeight: 'bold', letterSpacing: '-0.01em', lineHeight: '[1.4]' })}>
            {$entityView.node.title}
          </h1>

          {#if $entityView.node.subtitle}
            <p class={css({ marginTop: '8px', fontSize: '15px', color: 'text.muted' })}>
              {$entityView.node.subtitle}
            </p>
          {/if}

          <div class={flex({ align: 'center', justify: 'space-between', marginTop: '24px', paddingBottom: '16px' })}>
            <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'text.faint' })}>
              {#if $entityView.node.allowReaction && $entityView.node.reactions.length > 0}
                <div class={flex({ align: 'center', gap: '3px' })}>
                  <Icon icon={SmileIcon} />
                  <span>{comma($entityView.node.reactions.length)}</span>
                </div>
              {/if}
            </div>

            <div class={flex({ align: 'center', marginLeft: 'auto', gap: '12px', color: 'text.muted' })}>
              <ShareLinkPopover href={$entityView.url} />

              <PostActionMenu {$entityView} />
            </div>
          </div>

          <HorizontalDivider style={css.raw({ marginBottom: '24px' })} />
        </div>

        {#if $entityView.node.body.__typename === 'PostViewBodyAvailable'}
          {#if $entityView.node.protectContent}
            <ContentProtect>
              <TiptapRenderer style={css.raw({ width: 'full' })} content={$entityView.node.body.content} />
            </ContentProtect>
          {:else}
            <TiptapRenderer style={css.raw({ width: 'full' })} content={$entityView.node.body.content} />
          {/if}

          <div
            class={flex({
              align: 'flex-start',
              justify: 'space-between',
              gap: '8px',
              marginTop: '20px',
              paddingBottom: '10px',
              width: 'full',
              maxWidth: 'var(--prosemirror-max-width)',
            })}
          >
            <EmojiReaction $postView={$entityView.node} />

            <div class={flex({ align: 'center', gap: '12px', marginLeft: 'auto', color: 'text.muted' })}>
              <ShareLinkPopover href={$entityView.url} />

              <PostActionMenu {$entityView} />
            </div>
          </div>
        {:else if $entityView.node.body.__typename === 'PostViewBodyUnavailable'}
          <div class={css({ marginTop: '42px', fontSize: '16px', fontWeight: 'medium' })}>
            {#if $entityView.node.body.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
              <PostViewBodyUnavailable description="본인 인증이 필요한 글이에요" icon={ShieldAlertIcon} title="연령제한글">
                {#if $user}
                  <Button style={css.raw({ width: 'full' })} onclick={handleVerification} variant="secondary">본인 인증</Button>
                {:else}
                  <Button style={css.raw({ width: 'full' })} external href={authorizeUrl} type="link" variant="secondary">
                    로그인 후 본인 인증하기
                  </Button>
                {/if}
              </PostViewBodyUnavailable>
            {:else if $entityView.node.body.reason === 'REQUIRE_MINIMUM_AGE'}
              <PostViewBodyUnavailable
                description="이 글은 연령 기준에 따라 현재 계정으로는 열람이 제한되어 있어요"
                icon={ShieldAlertIcon}
                title="연령제한글"
              />
            {:else if $entityView.node.body.reason === 'REQUIRE_PASSWORD'}
              <form onsubmit={form.handleSubmit}>
                <PostViewBodyUnavailable description="해당 내용은 비밀번호 입력이 필요해요" icon={LockIcon} title="비밀글">
                  <div class={flex({ direction: 'column', gap: '4px' })}>
                    <TextInput
                      id="password"
                      style={css.raw({ width: 'full', height: '36px' })}
                      placeholder="비밀번호를 입력하세요"
                      bind:value={form.fields.password}
                    />

                    {#if form.errors.password}
                      <p class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.password}</p>
                    {/if}
                  </div>

                  <Button style={css.raw({ marginTop: '8px', width: 'full' })} type="submit">확인</Button>
                </PostViewBodyUnavailable>
              </form>
            {:else}
              {$entityView.node.body.reason}
            {/if}
          </div>
        {/if}

        <ContentNavigation {$entityView} />
      </div>
    </div>
  </div>
{/if}

<Modal style={css.raw({ padding: '24px', maxWidth: '440px' })} bind:open={paywallModalOpen}>
  {#if showCardRegistration}
    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <button
          class={css({ padding: '4px', color: 'text.muted', cursor: 'pointer', _hover: { color: 'text.default' } })}
          onclick={() => (showCardRegistration = false)}
          type="button"
        >
          <Icon icon={ChevronLeftIcon} size={20} />
        </button>
        <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>결제 수단 등록</h2>
      </div>

      <form class={flex({ flexDirection: 'column', gap: '16px' })} onsubmit={cardForm.handleSubmit}>
        <div class={flex({ flexDirection: 'column', gap: '8px' })}>
          <TextInput
            style={css.raw({ width: 'full' })}
            inputmode="numeric"
            maxlength={19}
            oninput={formatCardNumber}
            placeholder="카드 번호"
            bind:value={cardForm.fields.cardNumber}
          />

          <div class={flex({ gap: '8px' })}>
            <TextInput
              style={css.raw({ flex: '1' })}
              inputmode="numeric"
              maxlength={5}
              oninput={formatCardExpiry}
              placeholder="유효기간 (MM/YY)"
              bind:value={cardForm.fields.expiryDate}
            />
            <TextInput
              style={css.raw({ flex: '1' })}
              autocomplete="off"
              inputmode="numeric"
              maxlength={2}
              placeholder="비밀번호 앞 2자리"
              type="password"
              bind:value={cardForm.fields.passwordTwoDigits}
            />
          </div>

          <TextInput
            style={css.raw({ width: 'full' })}
            inputmode="numeric"
            maxlength={12}
            oninput={formatBusinessNumber}
            placeholder="생년월일 6자리 또는 사업자번호 10자리"
            bind:value={cardForm.fields.birthOrBusinessRegistrationNumber}
          />
        </div>

        <div class={flex({ flexDirection: 'column', gap: '8px' })}>
          <div
            class={css({
              borderRadius: '8px',
              borderWidth: '1px',
              borderColor: 'border.subtle',
              padding: '16px',
              backgroundColor: 'surface.default',
            })}
          >
            <div class={flex({ flexDirection: 'column', gap: '12px' })}>
              <Checkbox checked={allCardAgreementsChecked} onchange={handleAllCardAgreementCheck} size="sm">
                <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>전체 동의</span>
              </Checkbox>

              <div class={css({ height: '1px', backgroundColor: 'border.subtle' })}></div>

              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each cardAgreements as agreement, i (agreement.name)}
                  <Checkbox size="sm" bind:checked={cardAgreementChecks[i]}>
                    <span class={css({ fontSize: '13px', color: 'text.subtle' })}>
                      <a
                        class={css({ color: 'text.default', textDecoration: 'underline', _hover: { color: 'accent.brand.default' } })}
                        href={agreement.url}
                        rel="noopener noreferrer"
                        target="_blank"
                      >
                        {agreement.name}
                      </a>
                      동의 (필수)
                    </span>
                  </Checkbox>
                {/each}
              </div>
            </div>
          </div>

          {#if cardForm.errors.agreementsAccepted}
            <p class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{cardForm.errors.agreementsAccepted}</p>
          {/if}
        </div>

        {#if cardSubmitError}
          <div
            class={css({
              padding: '12px',
              borderRadius: '6px',
              backgroundColor: 'accent.danger.subtle',
              borderWidth: '1px',
              borderColor: 'border.danger',
            })}
          >
            <p class={css({ fontSize: '13px', color: 'text.danger' })}>{cardSubmitError}</p>
          </div>
        {/if}

        <Button style={css.raw({ width: 'full' })} type="submit">등록하기</Button>
      </form>
    </div>
  {:else}
    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>유료 콘텐츠 결제</h2>

      <p class={css({ fontSize: '14px', color: 'text.subtle', lineHeight: '[1.6]' })}>
        이 콘텐츠를 보려면 {comma(paywallPrice ?? 0)} P가 필요해요.
      </p>

      <div
        class={css({
          padding: '12px',
          borderRadius: '6px',
          backgroundColor: 'surface.subtle',
          fontSize: '13px',
          color: 'text.muted',
          lineHeight: '[1.6]',
        })}
      >
        <ul class={css({ paddingLeft: '16px', listStyleType: 'disc' })}>
          <li>디지털 콘텐츠의 특성상 결제 후에는 환불이 어려워요.</li>
          <li>결제한 콘텐츠는 현재 로그인한 계정에서만 볼 수 있어요.</li>
          <li>작성자가 콘텐츠를 삭제하면 더 이상 열람할 수 없어요.</li>
        </ul>
      </div>

      {#if hasBillingKey}
        <Button
          style={css.raw({ width: 'full' })}
          onclick={async () => {
            if ($entityView.node.__typename !== 'PostView' || !paywallNodeId) {
              return;
            }

            try {
              await purchasePaywall({
                postId: $entityView.node.id,
                nodeId: paywallNodeId,
              });

              mixpanel.track('purchase_paywall', {
                postId: $entityView.node.id,
                nodeId: paywallNodeId,
                price: paywallPrice,
              });

              cache.invalidate({ __typename: 'PostView', id: $entityView.node.id, field: 'body' });
              paywallModalOpen = false;
            } catch (err) {
              if (err instanceof TypieError) {
                Toast.error(err.message);
              }
            }
          }}
        >
          {comma(paywallPrice ?? 0)} P 결제하기
        </Button>
      {:else}
        <div class={flex({ flexDirection: 'column', gap: '8px' })}>
          <p class={css({ fontSize: '13px', color: 'text.muted', textAlign: 'center' })}>
            결제를 진행하려면 먼저 결제 수단을 등록해주세요.
          </p>
          <Button style={css.raw({ width: 'full' })} onclick={() => (showCardRegistration = true)}>결제 수단 등록하기</Button>
        </div>
      {/if}
    </div>
  {/if}
</Modal>
