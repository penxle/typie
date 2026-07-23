<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { graphql } from '$mearie';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const theme = getThemeContext();

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_OnboardingModal_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
          preferences
        }
      }
    `),
  );

  type LottieAsset = 'logo' | 'writing' | 'features';

  const lottieLoaders: Record<`${LottieAsset}_${'light' | 'dark'}`, () => Promise<{ default: object }>> = {
    logo_light: () => import('$assets/lottie/onboarding_logo_light.json'),
    logo_dark: () => import('$assets/lottie/onboarding_logo_dark.json'),
    writing_light: () => import('$assets/lottie/onboarding_writing_light.json'),
    writing_dark: () => import('$assets/lottie/onboarding_writing_dark.json'),
    features_light: () => import('$assets/lottie/onboarding_features_light.json'),
    features_dark: () => import('$assets/lottie/onboarding_features_dark.json'),
  };

  const pages: { asset: LottieAsset; loop: boolean; heroFraction: number; title: string; subtitle: string }[] = [
    {
      asset: 'logo',
      loop: false,
      heroFraction: 0.55,
      title: '타이피에 오신 걸 환영해요',
      subtitle: '떠오른 순간을 놓치지 않도록,\n언제 어디서나 편안하게 글을 이어 써보세요.',
    },
    {
      asset: 'writing',
      loop: true,
      heroFraction: 1,
      title: '글을 쓰는 모든 순간을 한곳에서',
      subtitle: '작품과 설정을 스페이스로 정리하고,\n나에게 맞는 환경에서 쓰고 공유해 보세요.',
    },
    {
      asset: 'features',
      loop: true,
      heroFraction: 1,
      title: '14일 무료 체험이 시작됐어요',
      subtitle: '타이피의 모든 기능을 이용할 수 있어요.\n지금 바로 첫 글을 시작해보세요.',
    },
  ];

  const dotIndices = pages.map((_, i) => i);

  let index = $state(0);
  let direction = $state(1);
  const page = $derived(pages[index]);
  const isLast = $derived(index === pages.length - 1);

  const goTo = (next: number) => {
    if (next === index || next < 0 || next >= pages.length) {
      return;
    }

    direction = next > index ? 1 : -1;
    index = next;
  };

  let lottieContainer = $state<HTMLDivElement>();

  $effect(() => {
    const container = lottieContainer;
    if (!container || !open) {
      return;
    }

    const current = page;
    const key = `${current.asset}_${theme.effectiveTheme}` as const;

    let animation: { destroy: () => void } | undefined;
    let cancelled = false;

    Promise.all([import('lottie-web'), lottieLoaders[key]()]).then(([{ default: lottie }, mod]) => {
      if (cancelled) {
        return;
      }

      animation = lottie.loadAnimation({
        container,
        renderer: 'svg',
        loop: current.loop,
        autoplay: true,
        animationData: mod.default,
      });
    });

    return () => {
      cancelled = true;
      animation?.destroy();
    };
  });

  const complete = () => {
    open = false;
    mixpanel.track('complete_onboarding');
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    updatePreferences({ input: { value: { webOnboardingCompletedAt: dayjs().toISOString() } } }).catch(() => {});
  };
</script>

<Modal
  style={css.raw({ alignItems: 'center', paddingTop: '26px', paddingX: '28px', paddingBottom: '22px', maxWidth: '400px' })}
  closable={false}
  bind:open
>
  <button
    class={css({
      position: 'absolute',
      top: '14px',
      left: '18px',
      fontSize: '12px',
      color: 'text.faint',
      cursor: 'pointer',
      userSelect: 'none',
      visibility: index === 0 ? 'hidden' : 'visible',
      _hover: { color: 'text.muted' },
    })}
    onclick={() => goTo(index - 1)}
    type="button"
  >
    이전
  </button>

  <button
    class={css({
      position: 'absolute',
      top: '14px',
      right: '18px',
      fontSize: '12px',
      color: 'text.faint',
      cursor: 'pointer',
      userSelect: 'none',
      visibility: isLast ? 'hidden' : 'visible',
      _hover: { color: 'text.muted' },
    })}
    onclick={() => {
      mixpanel.track('skip_onboarding');
      goTo(pages.length - 1);
    }}
    type="button"
  >
    건너뛰기
  </button>

  {#key index}
    <div
      class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full' })}
      in:fly={{ x: 24 * direction, duration: 250, easing: cubicOut }}
    >
      <div class={css({ display: 'flex', alignItems: 'center', justifyContent: 'center', width: 'full', height: '160px' })}>
        <div
          bind:this={lottieContainer}
          style:height={`${page.heroFraction * 100}%`}
          style:width={page.heroFraction === 1 ? '100%' : 'auto'}
          style:aspect-ratio={page.heroFraction === 1 ? undefined : '1'}
          class={css({ borderRadius: '12px', overflow: 'hidden' })}
        ></div>
      </div>

      <div class={css({ marginTop: '18px', fontSize: '17px', fontWeight: 'bold', color: 'text.default', textAlign: 'center' })}>
        {page.title}
      </div>

      <div class={css({ marginTop: '6px', fontSize: '13px', color: 'text.muted', textAlign: 'center', whiteSpace: 'pre-line' })}>
        {page.subtitle}
      </div>
    </div>
  {/key}

  <div class={flex({ gap: '4px', justifyContent: 'center', marginY: '12px' })}>
    {#each dotIndices as i (i)}
      <button
        class={css({ paddingX: '2px', paddingY: '4px', cursor: 'pointer' })}
        aria-label={`${i + 1}번째 페이지로 이동`}
        onclick={() => goTo(i)}
        type="button"
      >
        <div
          class={css({
            width: i === index ? '18px' : '6px',
            height: '6px',
            borderRadius: 'full',
            backgroundColor: i === index ? 'accent.brand.default' : 'interactive.hover',
            transition: 'common',
          })}
        ></div>
      </button>
    {/each}
  </div>

  <Button
    style={css.raw({ width: 'full' })}
    onclick={() => {
      if (isLast) {
        complete();
      } else {
        goTo(index + 1);
      }
    }}
  >
    {isLast ? '첫 글 시작하기' : '다음'}
  </Button>
</Modal>
