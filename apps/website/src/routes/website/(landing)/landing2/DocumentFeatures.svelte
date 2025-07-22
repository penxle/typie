<script lang="ts">
  import { onMount } from 'svelte';
  import AnchorIcon from '~icons/lucide/anchor';
  import FolderTreeIcon from '~icons/lucide/folder-tree';
  import SearchIcon from '~icons/lucide/search';
  import ShapesIcon from '~icons/lucide/shapes';
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
        <Icon icon={FolderTreeIcon} size={16} />
        ORGANIZATION
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
        쉽게 정리하고
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          편하게 찾기
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
        수많은 글과 자료도 체계적으로 관리하면 찾기 쉽습니다.
        <br />
        다양한 편의 기능으로 생각을 깔끔하게 정리해 보세요.
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
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={FolderTreeIcon} size={24} />
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
          폴더로 정리하기
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          폴더를 만들어 포스트와 자료를 주제별로 깔끔하게 분류하고 관리할 수 있습니다. 중첩 폴더로 더 체계적으로 정리하세요.
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
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={AnchorIcon} size={24} />
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
          앵커 설정
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          긴 글 안에서 중요한 부분에 앵커를 설정해두면, 스크롤 없이 한 번에 원하는 위치로 이동할 수 있습니다.
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
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={SearchIcon} size={24} />
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
          통합 검색
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          포스트 제목, 폴더명, 본문 내용까지 한 번에 검색하여 원하는 정보를 빠르게 찾을 수 있습니다.
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
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={ShapesIcon} size={24} />
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
          템플릿 활용
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          자주 사용하는 글의 형식을 템플릿으로 만들어두면, 반복을 줄이고 더 빨리 글쓰기를 시작할 수 있습니다.
        </p>
      </div>
    </div>
  </div>
</section>
