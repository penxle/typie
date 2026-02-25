<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Switch } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import type { DashboardLayout_PreferenceModal_AiTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_AiTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_AiTab_user on User {
        id
        preferences
      }
    `),
    () => user$key,
  );

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_AiTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
          preferences
        }
      }
    `),
  );

  let aiOptIn = $derived(user.data.preferences.aiOptIn ?? false);

  const handleToggle = () => {
    if (aiOptIn) {
      updatePreferences({ input: { value: { aiOptIn: false } } });
      mixpanel.track('ai_opt_in', { enabled: false });
    } else {
      Dialog.confirm({
        title: 'AI 기능을 활성화하시겠어요?',
        message:
          '사용자의 글은 AI 모델 학습에 절대 사용되지 않으며, 사용자가 요청할 때만 AI가 사용돼요. 언제든지 설정에서 비활성화할 수 있어요.',
        action: 'primary',
        actionLabel: '활성화',
        actionHandler: async () => {
          await updatePreferences({ input: { value: { aiOptIn: true } } });
          mixpanel.track('ai_opt_in', { enabled: true });
        },
      });
    }
  };
</script>

<div class={flex({ direction: 'column', maxWidth: '640px' })}>
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>AI</h1>
  </div>

  <div
    class={css({
      padding: '20px',
      borderRadius: '12px',
      backgroundColor: 'surface.subtle',
      borderWidth: '1px',
      borderColor: 'border.default',
    })}
  >
    <h2 class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default', marginBottom: '16px' })}>
      타이피는 사용자의 글을 절대 학습하지 않아요
    </h2>

    <div class={flex({ direction: 'column', gap: '12px', fontSize: '14px', color: 'text.default' })}>
      <p>
        타이피는 사용자의 프라이버시를 최우선으로 생각해요. 사용자가 작성한 글은
        <span class={css({ fontWeight: 'semibold' })}>어떠한 경우에도 AI 모델 학습에 사용되지 않아요.</span>
      </p>

      <ul class={css({ paddingLeft: '20px', listStyleType: 'disc' })}>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>학습 금지:</span>
          사용자의 글은 AI 모델 학습이나 개선에 절대 사용되지 않아요.
        </li>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>요청 시에만:</span>
          사용자가 요청하지 않는 한 타이피가 임의로 AI를 사용하지 않아요.
        </li>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>투명한 처리:</span>
          AI가 언제, 어떻게 사용되는지 사용자가 항상 알 수 있어요.
        </li>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>완전한 통제:</span>
          AI 기능은 언제든 끌 수 있고, 비활성화하면 어떤 AI 처리도 일어나지 않아요.
        </li>
        <li>
          <span class={css({ fontWeight: 'semibold' })}>권리 보장:</span>
          타이피는 사용자 창작물에 대한 어떤 권리도 주장하지 않아요.
        </li>
      </ul>
    </div>
  </div>

  <div class={css({ height: '20px' })}></div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        AI 기능 활성화
      {/snippet}
      {#snippet description()}
        활성화하면 AI 피드백 등 타이피가 제공하는 AI 기능을 사용할 수 있어요.
      {/snippet}
      {#snippet value()}
        <Switch
          checked={aiOptIn}
          onclick={(e) => {
            e.preventDefault();
            handleToggle();
          }}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
