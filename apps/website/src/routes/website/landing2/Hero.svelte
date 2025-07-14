<script lang="ts">
  import { animate, scroll } from 'motion';
  import { onMount } from 'svelte';
  import BoldIcon from '~icons/lucide/bold';
  import ImageIcon from '~icons/lucide/image';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import QuoteIcon from '~icons/lucide/quote';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import ActionBar from './ActionBar.svelte';
  import heroImage from './images/hero.png';
  import { Motion } from './motion.svelte';

  const distance = new Motion(100);

  onMount(() => {
    const element = document.querySelector('[data-element="root"]') as HTMLElement;

    return scroll(animate(distance.value, 0, { ease: 'easeIn' }), {
      offset: ['start start', 'start -500px'],
      target: element,
    });
  });
</script>

<div class={center({ flexDirection: 'column', width: 'full', position: 'relative', paddingY: '140px', zIndex: '50', overflow: 'hidden' })}>
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      overflow: 'hidden',
      backgroundImage: `[
        linear-gradient(to right, rgba(0, 0, 0, 0.05) 1px, transparent 1px),
        linear-gradient(to bottom, rgba(0, 0, 0, 0.05) 1px, transparent 1px)
      ]`,
      backgroundSize: '[40px 40px]',
      maskImage: '[radial-gradient(ellipse at center, black 0%, transparent 70%)]',
      pointerEvents: 'none',
    })}
  ></div>

  <h1
    class={css({
      fontSize: '[56px]',
      fontWeight: 'extrabold',
      color: 'gray.950',
      textAlign: 'center',
      fontFamily: 'Paperlogy',
      lineHeight: '[1.25]',
    })}
  >
    글쓰기의 모든 과정,
    <br />
    <span
      class={css({
        position: 'relative',
        _after: {
          content: '""',
          position: 'absolute',
          left: '-6px',
          right: '-6px',
          top: '0',
          bottom: '0',
          backgroundColor: 'amber.400',
          opacity: '25',
          zIndex: '[-1]',
        },
      })}
    >
      하나의 에디터로 완성하다
    </span>
  </h1>

  <p
    class={css({
      fontSize: '20px',
      fontWeight: 'medium',
      color: 'gray.700',
      textAlign: 'center',
      fontFamily: 'Pretendard',
      marginTop: '32px',
      marginBottom: '40px',
      lineHeight: '[1.5]',
    })}
  >
    쓰는 것이 즐거워지는 새로운 편집 경험.
    <br />
    흩어지고 끊기던 글쓰기의 흐름이, 이제는 자연스럽게 이어집니다.
  </p>

  <ActionBar />

  <div
    class={css({
      position: 'relative',
      marginTop: '56px',
      _before: {
        content: '""',
        position: 'absolute',
        top: '[-30%]',
        left: '[-20%]',
        right: '[-20%]',
        bottom: '[-30%]',
        background:
          '[radial-gradient(ellipse at center, color-mix(in oklch, {colors.amber.400} 20%, transparent), color-mix(in oklch, {colors.amber.400} 10%, transparent) 30%, color-mix(in oklch, {colors.amber.400} 5%, transparent) 60%, transparent 80%)]',
        filter: '[blur(60px)]',
        zIndex: '[-1]',
      },
    })}
  >
    {#each [{ icon: BoldIcon, angle: 125, distance: 650, rotate: '-5deg' }, { icon: ItalicIcon, angle: 130, distance: 900, rotate: '-10deg' }, { icon: ListIcon, angle: 150, distance: 700, rotate: '-20deg' }, { icon: LinkIcon, angle: 30, distance: 625, rotate: '25deg' }, { icon: ImageIcon, angle: 60, distance: 550, rotate: '10deg' }, { icon: QuoteIcon, angle: 55, distance: 850, rotate: '15deg' }] as iconConfig, index (index)}
      {@const x = Math.cos((iconConfig.angle * Math.PI) / 180) * iconConfig.distance * (distance.current / 100)}
      {@const y = Math.sin((iconConfig.angle * Math.PI) / 180) * iconConfig.distance * (distance.current / 100)}
      <div
        style:transform={`translate(calc(50% + ${x}px), calc(50% + ${-y}px)) translate(-50%, -50%) rotate(${iconConfig.rotate})`}
        class={css({
          position: 'absolute',
          top: '1/2',
          left: '1/2',
          width: '64px',
          height: '64px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: '[linear-gradient(135deg, rgba(255, 255, 255, 0.7) 0%, rgba(255, 255, 255, 0.5) 100%)]',
          borderRadius: '[20px]',
          boxShadow: '[0 20px 40px -10px rgba(0, 0, 0, 0.15), 0 10px 20px -5px rgba(0, 0, 0, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.5)]',
          border: '[1px solid rgba(255, 255, 255, 0.3)]',
          color: 'gray.800',
          backdropFilter: '[blur(20px) saturate(180%)]',
        })}
      >
        <Icon
          style={css.raw({
            '& * ': { strokeWidth: '[1.5px]' },
            color: 'gray.800',
          })}
          icon={iconConfig.icon}
          size={32}
        />
      </div>
    {/each}

    <div
      class={css({
        position: 'relative',
        padding: '16px',
        borderRadius: '[28px]',
        background: '[linear-gradient(135deg, rgba(255, 255, 255, 0.3) 0%, rgba(255, 255, 255, 0.2) 100%)]',
        backdropFilter: '[blur(10px)]',
        border: '[1px solid rgba(200, 200, 200, 0.8)]',
        boxShadow: '[0 8px 32px 0 rgba(31, 38, 135, 0.15)]',
        _before: {
          content: '""',
          position: 'absolute',
          inset: '0',
          borderRadius: '[28px]',
          padding: '1px',
          background: '[linear-gradient(135deg, rgba(255, 255, 255, 0.5) 0%, rgba(255, 255, 255, 0.1) 100%)]',
          mask: '[linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)]',
          maskComposite: 'exclude',
          zIndex: '[-1]',
        },
      })}
    >
      <img class={css({ borderRadius: '12px', width: '1024px', display: 'block' })} alt="hero" src={heroImage} />
    </div>
  </div>
</div>
