<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import PlayIcon from '~icons/lucide/play';
  import SquareIcon from '~icons/lucide/square';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import { AI_OPT_IN_FAILURE_MESSAGE, promptAiOptIn } from '../@prism/lib/ai-opt-in.ts';
  import { createPrismAudioPlayer } from '../@prism/prism-audio';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import type { DashboardLayout_PreferenceModal_PrismGeneralTab_user$key } from '$mearie';
  import type { PrismNotificationSound } from '../@prism/prism-notifications';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_PrismGeneralTab_user$key;
  };

  let { user$key }: Props = $props();
  const app = getAppContext();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_PrismGeneralTab_user on User {
        id
        preferences
      }
    `),
    () => user$key,
  );

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_PrismGeneralTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
          preferences
        }
      }
    `),
  );

  const persistedAiOptIn = $derived(user.data.preferences.aiOptIn ?? false);
  let audio: ReturnType<typeof createPrismAudioPlayer> | null = null;
  let playing = $state<PrismNotificationSound | null>(null);
  let aiOptInOverride = $state<boolean>();
  let updatingAiOptIn = $state(false);
  const aiOptIn = $derived(aiOptInOverride ?? persistedAiOptIn);
  const previewSounds = [
    { kind: 'resolved', label: '답변 완료' },
    { kind: 'action-required', label: '확인 필요' },
  ] satisfies { kind: PrismNotificationSound; label: string }[];

  $effect(() => {
    if (!updatingAiOptIn && aiOptInOverride !== undefined && aiOptInOverride === persistedAiOptIn) {
      aiOptInOverride = undefined;
    }
  });

  const updateAiOptIn = async (enabled: boolean) => {
    aiOptInOverride = enabled;
    updatingAiOptIn = true;
    try {
      await updatePreferences(
        { input: { value: { aiOptIn: enabled } } },
        enabled
          ? undefined
          : {
              metadata: {
                cache: {
                  optimisticResponse: {
                    updatePreferences: {
                      id: user.data.id,
                      preferences: { ...user.data.preferences, aiOptIn: enabled },
                    },
                  },
                },
              },
            },
      );
      mixpanel.track('ai_opt_in', { enabled, via: 'preferences_ai' });
    } catch {
      aiOptInOverride = undefined;
      Toast.error(AI_OPT_IN_FAILURE_MESSAGE);
    } finally {
      updatingAiOptIn = false;
    }
  };

  const handleToggle = () => {
    if (aiOptIn) {
      if (!SubscribeModal.gate('preferences_ai')) return;
      void updateAiOptIn(false);
      return;
    }

    promptAiOptIn(() => updateAiOptIn(true));
  };

  onMount(() => {
    const player = createPrismAudioPlayer();
    audio = player;
    return () => {
      player.destroy();
      if (audio === player) audio = null;
    };
  });

  const togglePreview = async (kind: PrismNotificationSound) => {
    if (playing === kind) {
      audio?.stop();
      playing = null;
      return;
    }

    playing = kind;
    const played =
      (await audio?.preview(kind, () => {
        if (playing === kind) playing = null;
      })) ?? false;
    if (!played && playing === kind) playing = null;
  };

  const sectionHeadingClass = css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '16px' });
</script>

<div class={flex({ direction: 'column', maxWidth: '640px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>일반</h1>

  <h2 class={sectionHeadingClass}>프리즘 활성화</h2>

  <div
    class={css({
      padding: '20px',
      borderRadius: '12px',
      backgroundColor: 'surface.subtle',
      borderWidth: '1px',
      borderColor: 'border.default',
    })}
  >
    <h3 class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default', marginBottom: '16px' })}>
      타이피는 사용자의 글을 절대 학습하지 않아요
    </h3>

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
      {#snippet label()}AI 기능 활성화{/snippet}
      {#snippet description()}활성화하면 AI 피드백 등 타이피가 제공하는 AI 기능을 사용할 수 있어요.{/snippet}
      {#snippet value()}
        <Switch
          checked={aiOptIn}
          disabled={updatingAiOptIn}
          onclick={(event) => {
            event.currentTarget.checked = aiOptIn;
            handleToggle();
          }}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>

  <div class={css({ height: '32px' })}></div>

  <h2 class={sectionHeadingClass}>알림</h2>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}알림음{/snippet}
      {#snippet description()}답변이나 확인 요청을 놓치기 쉬울 때 소리로 알려요.{/snippet}
      {#snippet value()}<Switch bind:checked={app.preference.current.prismNotificationSoundEnabled} />{/snippet}
    </SettingsRow>
  </SettingsCard>

  <div
    class={css({
      marginTop: '10px',
      borderRadius: '8px',
      paddingX: '12px',
      paddingY: '10px',
      backgroundColor: 'surface.subtle',
    })}
  >
    <p class={css({ paddingX: '4px', fontSize: '11px', fontWeight: 'medium', color: 'text.faint' })}>알림음 미리 듣기</p>
    <div class={flex({ gap: '4px', marginTop: '4px' })}>
      {#each previewSounds as sound (sound.kind)}
        <button
          class={flex({
            alignItems: 'center',
            gap: '7px',
            borderRadius: '6px',
            paddingX: '8px',
            paddingY: '6px',
            fontSize: '12px',
            color: playing === sound.kind ? 'text.default' : 'text.subtle',
            transition: 'common',
            _hover: { backgroundColor: 'surface.muted' },
          })}
          aria-label={`${sound.label} 알림음 ${playing === sound.kind ? '중지' : '재생'}`}
          aria-pressed={playing === sound.kind}
          onclick={() => void togglePreview(sound.kind)}
          type="button"
        >
          <span
            class={flex({
              alignItems: 'center',
              justifyContent: 'center',
              size: '22px',
              borderRadius: 'full',
              color: playing === sound.kind ? 'white' : 'text.faint',
              backgroundColor: playing === sound.kind ? 'accent.brand.default' : 'surface.muted',
            })}
          >
            <Icon icon={playing === sound.kind ? SquareIcon : PlayIcon} size={10} />
          </span>
          {sound.label}
        </button>
      {/each}
    </div>
  </div>

  <div class={css({ height: '32px' })}></div>

  <h2 class={sectionHeadingClass}>기타</h2>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}새 대화의 3D 프리즘{/snippet}
      {#snippet description()}새 프리즘 대화에서 3D 프리즘을 표시해요.{/snippet}
      {#snippet value()}<Switch bind:checked={app.preference.current.prismWelcomeObjectEnabled} />{/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
