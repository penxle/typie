<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import PartyPopperIcon from '~icons/lucide/party-popper';
  import XIcon from '~icons/lucide/x';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import SubscriptionCelebrationModal from './SubscriptionCelebrationModal.svelte';
  import type { TrialPopupExperimentModal_user$key } from '$mearie';

  type Props = {
    open: boolean;
    user$key: TrialPopupExperimentModal_user$key;
    documentId: string;
  };

  let { open = $bindable(false), user$key, documentId }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment TrialPopupExperimentModal_user on User {
        id
        canStartTrial

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const [recordSurvey] = createMutation(
    graphql(`
      mutation TrialPopupExperimentModal_RecordSurvey_Mutation($input: RecordSurveyInput!) {
        recordSurvey(input: $input) {
          id
        }
      }
    `),
  );

  const [subscribePlanWithTrial] = createMutation(
    graphql(`
      mutation TrialPopupExperimentModal_SubscribePlanWithTrial_Mutation {
        subscribePlanWithTrial {
          id
          state
          expiresAt

          plan {
            id
            name
            availability
          }
        }
      }
    `),
  );

  function resolveVariant(userId: string) {
    let hash = 2_166_136_261;

    for (const character of userId) {
      hash ^= character.codePointAt(0) ?? 0;
      hash = Math.imul(hash, 16_777_619);
    }

    return (hash >>> 0) % 2 === 0 ? 'A' : 'B';
  }

  const variant = $derived(resolveVariant(user.data.id));
  const canStartTrial = $derived(user.data.canStartTrial && !user.data.subscription);

  let viewRecorded = $state(false);
  let terminalActionRecorded = $state(false);
  let shownAt = $state<string | null>(null);
  let submitting = $state(false);
  let trialStartedModalOpen = $state(false);
  let viewRecordPromise: Promise<void> | null = null;

  const eventProperties = $derived({
    variant,
    trigger: 'document_body_focus',
    documentId,
  });

  async function persistAction(action: 'shown' | 'dismissed' | 'trial_started') {
    const updatedAt = new Date().toISOString();
    const persistedShownAt = shownAt ?? updatedAt;
    shownAt = persistedShownAt;

    await recordSurvey({
      input: {
        name: 'trial_popup_content_entry_202605',
        value: {
          variant,
          trigger: 'document_body_focus',
          action,
          documentId,
          shownAt: persistedShownAt,
          updatedAt,
        },
      },
    });
    cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'surveys' });
  }

  async function recordView() {
    shownAt = new Date().toISOString();
    viewRecorded = true;
    mixpanel.track('view_trial_popup_experiment', eventProperties);

    try {
      viewRecordPromise = persistAction('shown');
      await viewRecordPromise;
    } catch {
      // ignore
    }
  }

  async function handleDismiss() {
    if (terminalActionRecorded) {
      open = false;
      return;
    }

    terminalActionRecorded = true;
    mixpanel.track('dismiss_trial_popup_experiment', eventProperties);

    try {
      await viewRecordPromise?.catch(() => null);
      await persistAction('dismissed');
    } finally {
      open = false;
    }
  }

  async function handleStartTrial() {
    if (!canStartTrial || submitting) return;

    submitting = true;
    terminalActionRecorded = true;
    mixpanel.track('click_trial_popup_experiment_cta', eventProperties);

    try {
      await subscribePlanWithTrial();
    } catch {
      terminalActionRecorded = false;
      Toast.error('무료 체험을 시작하지 못했어요. 잠시 후 다시 시도해주세요.');
      submitting = false;
      return;
    }

    try {
      await viewRecordPromise?.catch(() => null);
      await persistAction('trial_started');
    } catch {
      // ignore
    } finally {
      submitting = false;
    }

    cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'subscription' });
    cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'canStartTrial' });
    cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'surveys' });
    mixpanel.track('start_trial', { via: 'trial_popup_experiment', variant });
    open = false;
    trialStartedModalOpen = true;
  }

  $effect(() => {
    if (open && !viewRecorded) {
      void recordView();
    }
  });
</script>

<Modal
  style={css.raw({
    width: { base: '[calc(100vw - 64px)]', sm: '[min(580px,calc(100vw - 112px))]' },
    maxWidth: '[580px]',
    padding: '0',
    borderRadius: { base: '[16px]', sm: '[20px]' },
    overflow: 'hidden',
    overflowY: 'hidden',
  })}
  onclose={() => void handleDismiss()}
  bind:open
>
  <button
    class={center({
      position: 'absolute',
      top: { base: '12px', sm: '18px' },
      right: { base: '12px', sm: '18px' },
      zIndex: '1',
      size: { base: '28px', sm: '32px' },
      borderRadius: 'full',
      color: 'dark.gray.300',
      transition: 'common',
      _hover: { color: 'dark.gray.50', backgroundColor: 'white/10' },
    })}
    aria-label="닫기"
    onclick={() => void handleDismiss()}
    type="button"
  >
    <Icon icon={XIcon} size={16} />
  </button>

  <div
    class={flex({
      flexDirection: 'column',
      width: 'full',
      minHeight: { base: '[386px]', sm: '[402px]' },
      aspectRatio: { sm: '[1005 / 698]' },
      backgroundColor: 'surface.default',
    })}
  >
    <div
      class={flex({
        flexDirection: 'column',
        alignSelf: 'stretch',
        alignItems: 'center',
        justifyContent: 'center',
        width: 'full',
        gap: { base: '12px', sm: '16px' },
        flex: '[0 0 42%]',
        paddingTop: { base: '[30px]', sm: '[36px]' },
        paddingX: { base: '20px', sm: '32px' },
        paddingBottom: { base: '[26px]', sm: '[32px]' },
        backgroundColor: 'dark.gray.950',
      })}
    >
      <Icon
        style={css.raw({
          color: 'accent.brand.default',
          width: { base: '[40px]', sm: '[48px]' },
          height: { base: '[40px]', sm: '[48px]' },
          strokeWidth: '[2.4px]',
        })}
        icon={PartyPopperIcon}
      />

      <h2
        class={css({
          color: 'dark.gray.50',
          fontFamily: 'Paperlogy',
          fontSize: { base: '[25px]', sm: '[32px]' },
          fontWeight: 'extrabold',
          lineHeight: '[1.12]',
          textAlign: 'center',
          wordBreak: 'keep-all',
        })}
      >
        첫 콘텐츠 생성을 축하드려요!
      </h2>
    </div>

    <div
      class={flex({
        flex: '1',
        flexDirection: 'column',
        alignSelf: 'stretch',
        alignItems: 'center',
        justifyContent: 'space-between',
        width: 'full',
        gap: { base: '24px', sm: '30px' },
        paddingTop: { base: '[32px]', sm: '[40px]' },
        paddingX: { base: '22px', sm: '44px' },
        paddingBottom: { base: '[28px]', sm: '[32px]' },
      })}
    >
      <p
        class={css({
          color: 'text.default',
          fontFamily: 'Paperlogy',
          fontSize: { base: '[22px]', sm: '[27px]' },
          fontWeight: 'extrabold',
          lineHeight: '[1.38]',
          textAlign: 'center',
          wordBreak: 'keep-all',
        })}
      >
        {#if variant === 'A'}
          커스텀 폰트로 <span class={css({ color: 'accent.brand.default' })}>나만의 글</span>
          을 만들고,
          <br />
          무제한 기능으로
          <span class={css({ color: 'accent.brand.default' })}>더 깊이 몰입</span>
          해보세요
        {:else}
          <span class={css({ color: 'accent.brand.default' })}>커스텀 폰트</span>
          로 가독성을 높이고
          <br />
          <span class={css({ color: 'accent.brand.default' })}>무제한 글자수, 무제한 파일 업로드</span>
          까지!
        {/if}
      </p>

      <Button
        style={css.raw({
          width: 'full',
          maxWidth: '[488px]',
          height: { base: '[48px]', sm: '[56px]' },
          borderRadius: 'full',
          backgroundColor: 'dark.gray.900',
          boxShadow: '[none]',
          color: 'dark.gray.50',
          fontFamily: 'Paperlogy',
          fontSize: { base: '[16px]', sm: '[20px]' },
          fontWeight: 'extrabold',
          letterSpacing: '0',
          _hover: { backgroundColor: 'dark.gray.800', color: 'dark.gray.50' },
          _active: { backgroundColor: 'dark.gray.900', color: 'dark.gray.50' },
        })}
        disabled={!canStartTrial}
        loading={submitting}
        onclick={() => void handleStartTrial()}
        size="lg"
        variant="ghost"
      >
        <div class={flex({ alignItems: 'center', justifyContent: 'center', gap: { base: '10px', sm: '14px' } })}>
          <span>모든 기능 2주 무료 체험하기</span>
          <Icon
            style={css.raw({
              flexShrink: '0',
              width: { base: '[22px]', sm: '[28px]' },
              height: { base: '[22px]', sm: '[28px]' },
              transition: 'transform',
              _groupHover: { transform: 'translateX(4px)' },
            })}
            icon={ChevronsRightIcon}
          />
        </div>
      </Button>
    </div>
  </div>
</Modal>

<SubscriptionCelebrationModal
  message="2주간 커스텀 폰트, 무제한 글자 수, 무제한 파일 업로드를 자유롭게 이용해보세요."
  title="무료 체험이 시작됐어요!"
  bind:open={trialStartedModalOpen}
/>
