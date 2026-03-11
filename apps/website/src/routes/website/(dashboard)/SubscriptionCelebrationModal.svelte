<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import CrownIcon from '~icons/lucide/crown';
  import GiftIcon from '~icons/lucide/gift';
  import KeyIcon from '~icons/lucide/key';
  import StarIcon from '~icons/lucide/star';
  import TagIcon from '~icons/lucide/tag';

  type Props = {
    open: boolean;
    title: string;
    message: string;
  };

  let { open = $bindable(false), title, message }: Props = $props();
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
    <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>{title}</div>

    <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>
      {message}
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
    <div class={flex({ justifyContent: 'center', alignItems: 'center', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>타이피 FULL ACCESS</div>
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
    style={css.raw({ marginTop: '24px', width: 'full' })}
    onclick={() => {
      open = false;
    }}
  >
    시작하기
  </Button>
</Modal>
