<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import TypeIcon from '~icons/lucide/book-open-text';
  import CrownIcon from '~icons/lucide/crown';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FlaskConicalIcon from '~icons/lucide/flask-conical';
  import GiftIcon from '~icons/lucide/gift';
  import HeadsetIcon from '~icons/lucide/headset';
  import ImagesIcon from '~icons/lucide/images';
  import KeyIcon from '~icons/lucide/key';
  import LinkIcon from '~icons/lucide/link';
  import SearchIcon from '~icons/lucide/search';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import SproutIcon from '~icons/lucide/sprout';
  import StarIcon from '~icons/lucide/star';
  import TagIcon from '~icons/lucide/tag';
  import TypeOutlineIcon from '~icons/lucide/type-outline';
  import { pushState } from '$app/navigation';
  import { Button, HorizontalDivider, Icon, Modal } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    open: boolean;
    children?: Snippet;
  };

  let { open = $bindable(false), children }: Props = $props();
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
    <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>플랜 업그레이드가 필요해요</div>

    <div class={css({ fontSize: '13px', color: 'text.faint' })}>
      {@render children?.()}
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
      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} size={14} />
        <span>무제한 글자 수</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={ImagesIcon} size={14} />
        <span>무제한 파일 업로드</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={SearchIcon} size={14} />
        <span>고급 검색</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={SpellCheckIcon} size={14} />
        <span>맞춤법 검사</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={LinkIcon} size={14} />
        <span>커스텀 게시 주소</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeOutlineIcon} size={14} />
        <span>커스텀 폰트 업로드</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={FlaskConicalIcon} size={14} />
        <span>베타 기능 우선 접근</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={HeadsetIcon} size={14} />
        <span>문제 발생시 우선 지원</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={SproutIcon} size={14} />
        <span>디스코드 커뮤니티 참여</span>
      </li>

      <li class={flex({ alignItems: 'center', gap: '6px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={EllipsisIcon} size={14} />
        <span>그리고 더 많은 혜택</span>
      </li>
    </ul>
  </div>

  <Button
    style={css.raw({ marginTop: '32px', width: 'full', height: '40px' })}
    gradient
    onclick={() => {
      open = false;
      pushState('', { shallowRoute: '/preference/billing' });
    }}
  >
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <span>업그레이드</span>

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
