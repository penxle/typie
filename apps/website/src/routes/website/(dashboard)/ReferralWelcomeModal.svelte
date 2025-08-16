<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CrownIcon from '~icons/lucide/crown';
  import GiftIcon from '~icons/lucide/gift';
  import KeyIcon from '~icons/lucide/key';
  import StarIcon from '~icons/lucide/star';
  import TagIcon from '~icons/lucide/tag';
  import { pushState } from '$app/navigation';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();
</script>

<Modal
  style={css.raw({
    alignItems: 'center',
    padding: '32px',
    maxWidth: '400px',
  })}
  bind:open
>
  <div
    class={flex({
      alignItems: 'center',
      '& > div': {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        borderWidth: '2px',
        borderColor: 'surface.default',
        borderRadius: 'full',
        marginRight: '-8px',
        size: '32px',
        color: 'text.bright',
        backgroundColor: 'surface.dark',
      },
    })}
  >
    <div>
      <Icon icon={CrownIcon} size={16} />
    </div>

    <div>
      <Icon icon={TagIcon} size={16} />
    </div>

    <div>
      <Icon icon={StarIcon} size={16} />
    </div>

    <div>
      <Icon icon={KeyIcon} size={16} />
    </div>

    <div>
      <Icon icon={GiftIcon} size={16} />
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', marginTop: '16px', textAlign: 'center' })}>
    <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>친구 초대로 가입하셨네요!</div>

    <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
      타이피 FULL ACCESS 플랜을 체험해볼 수 있는 4,900원 크레딧 혜택을 받았어요. 플랜을 업그레이드하면 나를 초대한 친구도 1달 무료 혜택을
      받아요.
    </div>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      marginTop: '24px',
      borderWidth: '1px',
      borderRadius: '8px',
      paddingX: '16px',
      paddingTop: '16px',
      paddingBottom: '32px',
      width: 'full',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>타이피 FULL ACCESS</div>

      <div class={css({ color: 'text.brand' })}>
        <span class={css({ fontSize: '15px', fontWeight: 'bold' })}>4,900</span>
        <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>원</span>
        <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>/ 월</span>
      </div>
    </div>

    <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

    <ul class={flex({ flexDirection: 'column', gap: '8px', fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>
      {#each PLAN_FEATURES.full as feature, index (index)}
        <li class={flex({ alignItems: 'center', gap: '6px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={feature.icon} size={14} />
          <span>{feature.label}</span>
        </li>
      {/each}
    </ul>
  </div>

  <Button
    style={css.raw({ marginTop: '32px', width: 'full', height: '40px' })}
    onclick={() => {
      open = false;
    }}
    variant="secondary"
  >
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <span>좀 더 둘러볼게요</span>
    </div>
  </Button>

  <Button
    style={css.raw({ marginTop: '8px', width: 'full', height: '40px' })}
    gradient
    onclick={() => {
      open = false;
      pushState('', { shallowRoute: '/preference/billing' });
    }}
  >
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <span>지금 업그레이드</span>

      <Icon
        style={css.raw({
          transition: 'transform',
          _groupHover: { transform: 'translateX(2px)' },
        })}
        icon={ArrowRightIcon}
        size={16}
      />
    </div>
  </Button>
</Modal>
