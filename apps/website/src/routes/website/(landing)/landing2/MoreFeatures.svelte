<script lang="ts">
  import { onMount } from 'svelte';
  import GlobeIcon from '~icons/lucide/globe';
  import MoonIcon from '~icons/lucide/moon';
  import Share2Icon from '~icons/lucide/share-2';
  import SmartphoneIcon from '~icons/lucide/smartphone';
  import UsersIcon from '~icons/lucide/users';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';

  let elements = $state<HTMLElement[]>([]);

  onMount(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('in-view');
          }
        });
      },
      {
        threshold: 0.1,
        rootMargin: '0px 0px 50px 0px',
      },
    );

    elements.forEach((element) => {
      if (element) observer.observe(element);
    });

    return () => {
      elements.forEach((element) => {
        if (element) observer.unobserve(element);
      });
    };
  });
</script>

<section
  class={css({
    position: 'relative',
    paddingX: '24px',
    paddingY: '120px',
    backgroundColor: 'gray.50',
    borderBottom: '8px solid',
    borderColor: 'gray.900',
  })}
>
  <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
    <div
      bind:this={elements[0]}
      class={center({
        flexDirection: 'column',
        marginBottom: '80px',
        opacity: '0',
        transform: 'translateY(20px) rotate(-1deg)',
        transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        '&.in-view': {
          opacity: '100',
          transform: 'translateY(0) rotate(0)',
        },
      })}
    >
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          backgroundColor: 'gray.900',
          color: 'white',
          paddingX: '20px',
          paddingY: '8px',
          fontSize: '14px',
          fontWeight: 'bold',
          marginBottom: '40px',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: 'rotate(-2deg)',
        })}
      >
        <Icon icon={GlobeIcon} size={16} />
        CONNECTION & EXTENSION
      </div>

      <h2
        class={css({
          fontSize: { base: '[48px]', md: '[64px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '24px',
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        함께,
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          언제 어디서나
        </span>
      </h2>
      <p
        class={css({
          fontSize: { base: '18px', md: '20px' },
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '[700px]',
          lineHeight: '[1.6]',
        })}
      >
        혼자가 아닌 함께, 한 곳이 아닌 어디서나.
        <br />
        글쓰기의 경계를 넘어 더 넓은 가능성을 만나보세요.
      </p>
    </div>

    <div
      bind:this={elements[1]}
      class={css({
        display: 'grid',
        gridTemplateColumns: { base: '1fr', md: '1fr 1fr' },
        gap: '32px',
        marginBottom: '32px',
        opacity: '0',
        transform: 'translateY(20px)',
        transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
        '&.in-view': {
          opacity: '100',
          transform: 'translateY(0)',
        },
      })}
    >
      <div
        class={css({
          backgroundColor: 'white',
          padding: { base: '32px', md: '48px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          position: 'relative',
          transform: 'rotate(-0.5deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={center({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={UsersIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '24px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '16px',
            textTransform: 'uppercase',
          })}
        >
          동시 편집
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          여러 사람이 동시에 하나의 문서를 편집할 수 있습니다. 실시간으로 변경사항이 반영되어 효율적인 협업이 가능합니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'gray.900',
          padding: { base: '32px', md: '40px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #fbbf24]',
          position: 'relative',
          transform: 'rotate(0.5deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #fbbf24]',
          },
        })}
      >
        <div
          class={center({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={Share2Icon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '24px',
            fontWeight: 'black',
            color: 'white',
            marginBottom: '16px',
            textTransform: 'uppercase',
          })}
        >
          링크 공유
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          링크 하나로 쉽게 글을 공유하세요. 읽기 전용, 편집 가능 등 권한을 설정하고 비밀번호로 보호할 수 있습니다.
        </p>
      </div>
    </div>

    <div
      bind:this={elements[2]}
      class={css({
        display: 'grid',
        gridTemplateColumns: { base: '1fr', md: '1fr 1fr' },
        gap: '32px',
        opacity: '0',
        transform: 'translateY(20px)',
        transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s]',
        '&.in-view': {
          opacity: '100',
          transform: 'translateY(0)',
        },
      })}
    >
      <div
        class={css({
          backgroundColor: 'white',
          padding: { base: '32px', md: '48px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          position: 'relative',
          transform: 'rotate(0.3deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={center({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={MoonIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '24px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '16px',
            textTransform: 'uppercase',
          })}
        >
          다크 모드
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          눈의 피로를 줄이는 다크모드를 지원합니다. 시간대에 따라 자동으로 전환되도록 설정할 수도 있습니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: { base: '32px', md: '48px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          position: 'relative',
          transform: 'rotate(-0.3deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={center({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={SmartphoneIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '24px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '16px',
            textTransform: 'uppercase',
          })}
        >
          모바일 앱
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          iOS와 Android 앱으로 언제 어디서나 글을 쓸 수 있습니다. 모든 기기에서 자동으로 동기화됩니다.
        </p>
      </div>
    </div>
  </div>
</section>
