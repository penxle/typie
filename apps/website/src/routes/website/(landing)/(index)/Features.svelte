<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import BookMarkedIcon from '~icons/lucide/book-marked';
  import CalendarDaysIcon from '~icons/lucide/calendar-days';
  import CodeIcon from '~icons/lucide/code';
  import FolderHeartIcon from '~icons/lucide/folder-heart';
  import GlobeIcon from '~icons/lucide/globe';
  import ImageIcon from '~icons/lucide/image';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import PencilRulerIcon from '~icons/lucide/pencil-ruler';
  import QuoteIcon from '~icons/lucide/quote';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import SaveIcon from '~icons/lucide/save';
  import ShapesIcon from '~icons/lucide/shapes';
  import SparklesIcon from '~icons/lucide/sparkles';
  import TableIcon from '~icons/lucide/table';
  import UsersRoundIcon from '~icons/lucide/users-round';
</script>

<section
  class={css({
    position: 'relative',
    paddingX: { sm: '16px', lg: '24px' },
    paddingY: { sm: '80px', lg: '120px' },
    backgroundColor: 'gray.50',
    borderTop: '8px solid',
    borderBottom: '8px solid',
    borderColor: 'gray.900',
  })}
>
  <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
    <div
      class={center({
        flexDirection: 'column',
        marginBottom: { sm: '60px', lg: '80px' },
        opacity: '0',
        transform: { sm: 'translate3d(0, 20px, 0)', lg: 'translate3d(0, 20px, 0) rotate(-1deg)' },
        transition: {
          sm: '[opacity 0.3s ease-out, transform 0.3s ease-out]',
          lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        },
        willChange: 'transform, opacity',
        '&.in-view': {
          opacity: '100',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'translate3d(0, 0, 0) rotate(0)' },
        },
      })}
      data-observe
    >
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          backgroundColor: 'gray.900',
          color: 'white',
          paddingX: { sm: '16px', lg: '20px' },
          paddingY: { sm: '6px', lg: '8px' },
          fontSize: { sm: '12px', lg: '14px' },
          fontWeight: 'bold',
          marginBottom: { sm: '24px', lg: '40px' },
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: { sm: 'rotate(0)', lg: 'rotate(-2deg)' },
        })}
      >
        FEATURES
      </div>

      <h2
        class={css({
          fontSize: { sm: '[36px]', lg: '[64px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: { sm: '16px', lg: '24px' },
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        글쓰기에 필요한
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: { sm: '16px', lg: '20px' },
            display: 'inline-block',
            transform: { sm: 'rotate(0)', lg: 'rotate(1deg)' },
          })}
        >
          모든 것
        </span>
      </h2>
      <p
        class={css({
          fontSize: { sm: '16px', lg: '20px' },
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          maxWidth: '[800px]',
          lineHeight: '[1.6]',
          marginX: 'auto',
        })}
      >
        수많은 원고를 쓰고 고치며 생각했던 내용을 모두 담았어요.
        <br />
        타이핑부터 저장까지, 세심하게 다듬은 글쓰기의 모든 과정을 경험해보세요.
      </p>
    </div>

    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '2fr 1fr' },
        gap: { sm: '24px', lg: '32px' },
        marginBottom: { sm: '24px', lg: '32px' },
        opacity: '0',
        transform: 'translate3d(0, 20px, 0)',
        transition: {
          sm: '[opacity 0.3s ease-out 0.1s, transform 0.3s ease-out 0.1s]',
          lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
        },
        willChange: 'transform, opacity',
        '&.in-view': {
          opacity: '100',
          transform: 'translate3d(0, 0, 0)',
        },
      })}
      data-observe
    >
      <div
        class={css({
          backgroundColor: 'white',
          padding: { sm: '24px', lg: '48px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          position: 'relative',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.3deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            position: 'absolute',
            top: { sm: '-12px', lg: '-16px' },
            right: { sm: '24px', lg: '32px' },
            backgroundColor: 'amber.400',
            color: 'gray.900',
            paddingX: { sm: '12px', lg: '16px' },
            paddingY: { sm: '4px', lg: '6px' },
            fontSize: { sm: '10px', lg: '12px' },
            fontWeight: 'black',
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
            border: '4px solid',
            borderColor: 'gray.900',
            transform: { sm: 'rotate(0)', lg: 'rotate(2deg)' },
            boxShadow: '[4px 4px 0 0 #000]',
          })}
        >
          EDITOR
        </div>

        <div
          class={css({
            display: 'grid',
            gridTemplateColumns: { sm: 'repeat(3, 1fr)', lg: 'repeat(6, 1fr)' },
            gap: { sm: '12px', lg: '16px' },
            marginBottom: { sm: '24px', lg: '32px' },
          })}
        >
          {#each [ImageIcon, LinkIcon, QuoteIcon, TableIcon, ListIcon, CodeIcon] as icon, index (index)}
            <div
              class={css({
                height: { sm: '48px', lg: '64px' },
                backgroundColor: 'gray.100',
                border: '3px solid',
                borderColor: 'gray.900',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                _hover: {
                  backgroundColor: 'amber.400',
                  transform: 'translate3d(0, -2px, 0)',
                },
              })}
            >
              <Icon style={css.raw({ color: 'gray.900' })} {icon} size={24} />
            </div>
          {/each}
        </div>

        <h3
          class={css({
            fontSize: { sm: '20px', lg: '24px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '12px', lg: '16px' },
            textTransform: 'uppercase',
          })}
        >
          방해 없이, 쓰는 데만 집중할 수 있어요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '16px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          매일 글을 쓰며 마주치는 사소한 불편함들을 줄였어요. 현재 줄이 항상 화면 중앙에 오도록 고정되는 타자기 모드, 커서를 기준으로 줄을
          강조해주는 시선 유도, 따옴표 자동 입력, 손에 익은 단축키와 눈이 편안한 다크 테마까지. 눈과 손, 커서가 자연스럽게 맞물리는 경험을
          제공해요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'gray.900',
          padding: { sm: '24px', lg: '40px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #fbbf24]',
          position: 'relative',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(0.2deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #fbbf24]', lg: '[12px 12px 0 0 #fbbf24]' },
          },
        })}
      >
        <div class={flex({ alignItems: 'center', gap: { sm: '8px', lg: '12px' }, marginBottom: { sm: '16px', lg: '24px' } })}>
          <Icon style={css.raw({ color: 'amber.400' })} icon={ShapesIcon} size={24} />
          <h3
            class={css({
              fontSize: { sm: '20px', lg: '24px' },
              fontWeight: 'black',
              color: 'white',
              textTransform: 'uppercase',
            })}
          >
            어떤 글이든, 어떤 방식이든
          </h3>
        </div>
        <p class={css({ fontSize: { sm: '14px', lg: '16px' }, color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          웹소설 연재, 출판 원고, 일기, 블로그 초안까지. 원하는 어떤 포맷의 글이든 쓸 수 있어요. 자주 쓰는 포맷은 템플릿으로 저장하고 언제든
          불러오세요.
        </p>
      </div>
    </div>

    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '1fr 1fr 1fr' },
        gap: { sm: '24px', lg: '32px' },
        opacity: '0',
        transform: 'translate3d(0, 20px, 0)',
        transition: {
          sm: '[opacity 0.3s ease-out 0.2s, transform 0.3s ease-out 0.2s]',
          lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s]',
        },
        willChange: 'transform, opacity',
        '&.in-view': {
          opacity: '100',
          transform: 'translate3d(0, 0, 0)',
        },
      })}
      data-observe
    >
      <div
        class={css({
          backgroundColor: 'white',
          padding: { sm: '24px', lg: '32px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.5deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon
            style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
            icon={RulerDimensionLineIcon}
            size={24}
          />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          언제나 원하는 느낌을 살려서
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          문단 간격, 들여쓰기, 본문 폭 등 원고 서식을 세밀하게 조정할 수 있어요. 기본 제공되는 폰트 중 원하는 느낌이 없다면, 가지고 있는
          폰트를 직접 업로드해서 쓸 수도 있어요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(0.1deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon
            style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
            icon={PencilRulerIcon}
            size={24}
          />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          복잡한 플롯도 한눈에 정리해요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          캔버스 기능을 활용하면 인물 관계, 사건의 흐름, 세계관 구조 등을 시각적으로 구성할 수 있어요. 긴 글을 쓰기 전 맥락과 전개를
          정리하는 데 제일 좋아요. 글 옆에 작은 메모를 남길 수 있는 작성 노트도 유용하게 이용하세요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(0.4deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon
            style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
            icon={FolderHeartIcon}
            size={24}
          />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          글과 자료를 함께 관리해요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          폴더를 이용해 글을 주제별로 정리하고, 참고용 메모나 시놉시스도 함께 보관할 수 있어요. 글과 관련된 모든 자료를 구조화해 한번에
          관리하세요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.1deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon
            style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
            icon={BookMarkedIcon}
            size={24}
          />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          글 속에서 헤매지 않아요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          검색 기능과 단축키로 여러 글을 빠르게 탐색할 수 있어요. 글 안에서는 앵커 기능으로 글 내에 미리 표시해둔 위치로 바로 이동할 수도
          있어요. 글이 많고 길어질수록 빛나는 기능이에요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.3deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon
            style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
            icon={UsersRoundIcon}
            size={24}
          />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          실시간으로 함께 쓰고, 공유해요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          링크 하나로 글을 공유하고, 타인과 동시에 편집할 수도 있어요. 공동 작성, 피드백 요청, 감수 작업까지 모두 효율적으로 진행해요.
          필요하다면 성인 인증이나 비밀번호를 걸어 내용을 보호할 수도 있어요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.4deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })} icon={SaveIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          언제나 안전하게 저장해요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          모든 글을 매 타이핑마다 실시간으로 브라우저와 서버에 자동 저장해요. 연결이 끊기거나 실수로 창을 닫아도, 바로 직전 상태로 복원할 수
          있어요. 중요한 작업일수록 안심하고 작성하세요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(0.1deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })} icon={GlobeIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          언제 어디서든, 끊기지 않고 이어서 써요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          PC 웹과 Android/iOS 앱 모두 지원하며, 작업 내용은 자동으로 실시간 동기화돼요. 집에서 쓰다 잠깐 나와서도, 침대에 누워서도 갑자기
          떠오른 문장을 바로 이어 쓸 수 있어요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'white',
          padding: '32px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.2deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
          },
        })}
      >
        <div
          class={css({
            width: { sm: '40px', lg: '48px' },
            height: { sm: '40px', lg: '48px' },
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: { sm: '16px', lg: '24px' },
            transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
          })}
        >
          <Icon
            style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
            icon={CalendarDaysIcon}
            size={24}
          />
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
          })}
        >
          매일의 글쓰기 흐름을 기록해요
        </h3>
        <p class={css({ fontSize: { sm: '14px', lg: '15px' }, color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          하루하루의 작성 기록을 자동으로 기록해요. 매일 얼마나 썼는지, 얼마나 지웠는지를 개별적으로 볼 수 있어요. 쓰는 리듬을 만들고,
          꾸준히 이어나가 달력을 칸칸이 채워보세요.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'gray.900',
          padding: { sm: '24px', lg: '32px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #fbbf24]',
          gridColumn: { sm: '1', lg: 'auto' },
          transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-0.6deg)' },
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          willChange: 'transform',
          _hover: {
            transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
            boxShadow: { sm: '[8px 8px 0 0 #fbbf24]', lg: '[12px 12px 0 0 #fbbf24]' },
          },
        })}
      >
        <div class={flex({ alignItems: 'center', justifyContent: 'center', marginBottom: { sm: '16px', lg: '24px' } })}>
          <div
            class={css({
              width: { sm: '40px', lg: '48px' },
              height: { sm: '40px', lg: '48px' },
              backgroundColor: 'amber.400',
              border: '3px solid',
              borderColor: 'gray.900',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
            })}
          >
            <Icon
              style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
              icon={SparklesIcon}
              size={24}
            />
          </div>
        </div>
        <h3
          class={css({
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'black',
            color: 'white',
            marginBottom: { sm: '8px', lg: '12px' },
            textTransform: 'uppercase',
            textAlign: 'center',
          })}
        >
          더 많은 기능
        </h3>
        <p
          class={css({
            fontSize: { sm: '14px', lg: '15px' },
            color: 'gray.300',
            lineHeight: '[1.6]',
            fontWeight: 'medium',
            textAlign: 'center',
          })}
        >
          계속해서 편리한 기능들이 추가되고 있어요!
        </p>
      </div>
    </div>
  </div>
</section>
