<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { subscriptionStateLabels, subscriptionStateTones } from '$lib/admin-labels';
  import { AdminBadge, AdminKeyValue, AdminSection } from '$lib/components/admin';
  import { graphql } from '$mearie';
  import type { AdminUserOverviewTab_user$key } from '$mearie';

  type Props = {
    user$key: AdminUserOverviewTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment AdminUserOverviewTab_user on User {
        id
        createdAt
        credit
        hasPassword
        marketingConsent
        documentCount

        usage {
          totalCharacterCount
          totalBlobSize
        }

        singleSignOns {
          id
          provider
          email
        }

        personalIdentity {
          id
          name
          birthDate
          gender
          phoneNumber
        }

        subscriptions {
          id
          state
          startsAt
          expiresAt

          plan {
            id
            name
          }
        }
      }
    `),
    () => user$key,
  );
</script>

{#snippet singleSignOns()}
  {#if user.data.singleSignOns.length === 0}
    <span>—</span>
  {:else}
    {#each user.data.singleSignOns as sso (sso.id)}
      <div>{sso.provider} · {sso.email}</div>
    {/each}
  {/if}
{/snippet}

{#snippet personalIdentity()}
  {#if user.data.personalIdentity}
    <div>
      {user.data.personalIdentity.name} · {dayjs(user.data.personalIdentity.birthDate).formatAsDate()} ·
      {user.data.personalIdentity.gender} · {user.data.personalIdentity.phoneNumber}
    </div>
  {:else}
    <span>—</span>
  {/if}
{/snippet}

{#snippet subscriptions()}
  {#if user.data.subscriptions.length === 0}
    <span class={css({ color: 'text.disabled' })}>—</span>
  {:else}
    <div class={css({ display: 'flex', flexDirection: 'column', gap: '6px' })}>
      {#each user.data.subscriptions as subscription (subscription.id)}
        <div class={css({ display: 'flex', alignItems: 'center', gap: '6px' })}>
          <AdminBadge
            label={subscription.plan.name}
            tone={subscription.state === 'ACTIVE' ? 'brand' : subscriptionStateTones[subscription.state]}
          />
          <span class={css({ fontSize: '12px', color: 'text.faint' })}>{subscriptionStateLabels[subscription.state]}</span>
          <span class={css({ fontSize: '12px', color: 'text.faint' })}>
            {dayjs(subscription.startsAt).formatAsDate()} ~ {dayjs(subscription.expiresAt).formatAsDate()}
          </span>
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

<div class={css({ display: 'grid', gridTemplateColumns: { sm: '1fr', lg: '[1fr 1fr]' }, gap: '32px' })}>
  <div class={css({ display: 'flex', flexDirection: 'column', gap: '28px' })}>
    <AdminSection title="결제">
      <AdminKeyValue
        items={[
          { label: '구독', content: subscriptions },
          { label: '크레딧', value: `${comma(user.data.credit)}원` },
        ]}
      />
    </AdminSection>

    <AdminSection title="사용">
      <AdminKeyValue
        items={[
          { label: '문서 수', value: comma(user.data.documentCount) },
          { label: '총 글자 수', value: comma(user.data.usage.totalCharacterCount) },
          { label: '저장 용량', value: `${comma(Number(user.data.usage.totalBlobSize))} bytes` },
        ]}
      />
    </AdminSection>
  </div>

  <div class={css({ display: 'flex', flexDirection: 'column', gap: '28px' })}>
    <AdminSection title="계정">
      <AdminKeyValue
        items={[
          { label: '가입일', value: dayjs(user.data.createdAt).formatAsDateTime() },
          { label: '비밀번호 로그인', value: user.data.hasPassword ? '가능' : '불가능' },
          { label: '로그인 수단', content: singleSignOns },
        ]}
      />
    </AdminSection>

    <AdminSection title="확인">
      <AdminKeyValue
        items={[
          { label: '본인인증', content: personalIdentity },
          { label: '마케팅 수신 동의', value: user.data.marketingConsent ? '동의' : '미동의' },
        ]}
      />
    </AdminSection>

    <AdminSection title="식별자">
      <AdminKeyValue items={[{ label: 'ID', value: user.data.id, mono: true }]} />
    </AdminSection>
  </div>
</div>
