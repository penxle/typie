<script lang="ts">
  import { onMount } from 'svelte';
  import CodeIcon from '~icons/lucide/code';
  import FileTextIcon from '~icons/lucide/file-text';
  import ImageIcon from '~icons/lucide/image';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import NotebookIcon from '~icons/lucide/notebook';
  import PaletteIcon from '~icons/lucide/palette';
  import PencilIcon from '~icons/lucide/pencil';
  import SettingsIcon from '~icons/lucide/settings';
  import SparklesIcon from '~icons/lucide/sparkles';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import TableIcon from '~icons/lucide/table';
  import TypeIcon from '~icons/lucide/type';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

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
    borderTop: '8px solid',
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
        <Icon icon={PencilIcon} size={16} />
        WRITING TOOLS
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
        글쓰기에 필요한
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          모든 도구
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
        본문 작성부터 아이디어 시각화까지, 글쓰기의 모든 과정을 지원합니다.
        <br />
        생각을 온전히 표현할 수 있는 완벽한 도구들이 준비되어 있습니다.
      </p>
    </div>

    <div
      bind:this={elements[1]}
      class={css({
        display: 'grid',
        gridTemplateColumns: { base: '1fr', lg: '2fr 1fr' },
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
          class={css({
            position: 'absolute',
            top: '-16px',
            right: '32px',
            backgroundColor: 'amber.400',
            color: 'gray.900',
            paddingX: '16px',
            paddingY: '6px',
            fontSize: '12px',
            fontWeight: 'black',
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
            border: '4px solid',
            borderColor: 'gray.900',
            transform: 'rotate(2deg)',
            boxShadow: '[4px 4px 0 0 #000]',
          })}
        >
          EDITOR
        </div>

        <div
          class={css({
            display: 'grid',
            gridTemplateColumns: 'repeat(6, 1fr)',
            gap: '16px',
            marginBottom: '32px',
          })}
        >
          {#each [ImageIcon, LinkIcon, TableIcon, ListIcon, CodeIcon, FileTextIcon] as icon, index (index)}
            <div
              class={css({
                height: '64px',
                backgroundColor: 'gray.100',
                border: '3px solid',
                borderColor: 'gray.900',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                _hover: {
                  backgroundColor: 'amber.400',
                  transform: 'translateY(-2px)',
                },
              })}
            >
              <Icon style={css.raw({ color: 'gray.900' })} {icon} size={24} />
            </div>
          {/each}
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
          강력한 에디터 도구
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          기본적인 서식부터 이미지, 링크, 표 삽입까지. 글을 풍부하게 만드는 데 필요한 모든 도구가 준비되어 있습니다.
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
        <div class={flex({ alignItems: 'center', gap: '12px', marginBottom: '24px' })}>
          <Icon style={css.raw({ color: 'amber.400' })} icon={PaletteIcon} size={24} />
          <h3
            class={css({
              fontSize: '24px',
              fontWeight: 'black',
              color: 'white',
              textTransform: 'uppercase',
            })}
          >
            캔버스
          </h3>
        </div>
        <p class={css({ fontSize: '16px', color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          자유롭게 그림을 그리고 다이어그램을 만들어 보세요. 아이디어를 시각적으로 표현하고 글과 함께 배치할 수 있습니다.
        </p>
      </div>
    </div>

    <div
      bind:this={elements[2]}
      class={css({
        display: 'grid',
        gridTemplateColumns: { base: '1fr', md: '1fr 1fr', lg: '1fr 1fr 1fr' },
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
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: 'rotate(-0.3deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={css({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={SpellCheckIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '20px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '12px',
            textTransform: 'uppercase',
          })}
        >
          맞춤법 검사
        </h3>
        <p class={css({ fontSize: '15px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          오탈자 걱정 없이 편하게 쓸 수 있도록, 맞춤법과 문법을 꼼꼼하게 살펴 교정합니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: 'rotate(0.3deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={css({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={TypeIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '20px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '12px',
            textTransform: 'uppercase',
          })}
        >
          폰트 선택
        </h3>
        <p class={css({ fontSize: '15px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          엄선된 기본 폰트는 물론, 원하는 폰트를 직접 업로드하여 사용할 수 있습니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: 'rotate(-0.5deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={css({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={NotebookIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '20px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '12px',
            textTransform: 'uppercase',
          })}
        >
          작성 노트
        </h3>
        <p class={css({ fontSize: '15px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          쓰기 중 떠오르는 아이디어나 메모를 포스트 옆에 바로 기록할 수 있습니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: 'rotate(0.5deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={css({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={SettingsIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '20px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '12px',
            textTransform: 'uppercase',
          })}
        >
          양식 설정
        </h3>
        <p class={css({ fontSize: '15px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          본문 폭, 들여쓰기, 문단 간격 등 세밀한 양식 설정으로 읽기 편한 환경을 만들 수 있습니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'gray.900',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #fbbf24]',
          gridColumn: { base: '1', md: 'span 2', lg: 'auto' },
          transform: 'rotate(-0.3deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #fbbf24]',
          },
        })}
      >
        <div class={flex({ alignItems: 'center', justifyContent: 'center', marginBottom: '24px' })}>
          <div
            class={css({
              width: '48px',
              height: '48px',
              backgroundColor: 'amber.400',
              border: '3px solid',
              borderColor: 'gray.900',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transform: 'rotate(45deg)',
            })}
          >
            <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={SparklesIcon} size={24} />
          </div>
        </div>
        <h3
          class={css({
            fontSize: '20px',
            fontWeight: 'black',
            color: 'white',
            marginBottom: '12px',
            textTransform: 'uppercase',
            textAlign: 'center',
          })}
        >
          더 많은 기능
        </h3>
        <p class={css({ fontSize: '15px', color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium', textAlign: 'center' })}>
          계속해서 추가되는 새로운 기능들로 글쓰기 경험을 향상시키세요.
        </p>
      </div>
    </div>
  </div>
</section>
