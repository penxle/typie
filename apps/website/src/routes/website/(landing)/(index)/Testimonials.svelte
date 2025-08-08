<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';

  type Testimonial = {
    avatar: string;
    author: string;
    handle: string;
    content: string;
    url: string;
  };

  // spell-checker:disable
  const testimonials: Testimonial[] = [
    {
      avatar: 'https://pbs.twimg.com/profile_images/1907998919797387266/wmZyRRj3_400x400.jpg',
      author: '백지',
      handle: '@paperr_white',
      content: '타이피가 좋다… 쓸데없는 플랫폼이 아니라 그냥 정말 글 쓰는 기능만 있어서 더 좋다…',
      url: 'https://x.com/paperr_white/status/1933026516767756512',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1947651866332172288/3kMyY1bb_400x400.jpg',
      author: '주성이',
      handle: '@izatsuyu_dream',
      content: '타이피는 정말 좋은 어플이다\n타이피어플출시되고나서 나의 연성생활달라졌다',
      url: 'https://x.com/izatsuyu_dream/status/1947480997010018709?s=46&t=CT-W3Ige047OuUmFWL09YA',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1941691464687230976/rhmYr0Ju_400x400.jpg',
      author: '8484',
      handle: '@aibo8484',
      content:
        '타이피 진짜 좋죠 퇴근길에 타이피를 생각해 보세요 집에서 기다리고 있는 마누라 같고 포근하고 말랑하고 달콤한 무언가를 떠올리게 해요',
      url: 'https://x.com/aibo8484/status/1945484259067863163',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1946559332167962624/8VjxAiKX_400x400.jpg',
      author: '쀼뀨쀼',
      handle: '@pp_ggpp',
      content: '사용 한줄평: 최고 장점은 PC-모바일 동시 작업 가능한 한국어 글쓰기앱 + 포스팅 기능\n평점: ★★★★☆(4.5/5.0)',
      url: 'https://x.com/pp_ggpp/status/1936411971525775724',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1912656734738632704/uWHFBpb__400x400.jpg',
      author: '킴쓔',
      handle: '@soosoo_mocha',
      content:
        '이번 마감을 빨리 끝낼 수 있었던 일등공신이었습니다 특히 저처럼 여러 기기를 옮겨가며 글을 쓰시는 분께 추천 드립니다. 모바일과 PC에서 동시에 열고 쓸 수도 있고, 자동저장 기능이 정말 훌륭합니다. 구독료가 아깝지 않아서 앞으로도 계속 쓸 계획입니다',
      url: 'https://x.com/soosoo_mocha/status/1939327966271697303',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1796169430214553600/BgeyWus2_400x400.jpg',
      author: '도라방스',
      handle: '@purpledora2',
      content:
        '저도 타이피를 아주 잘 쓰고 있어서 장점을 공유해 봅니다\n\n1. 우선 가장 좋은 점은 직관적인 인터페이스에요.\n내가 지금 어떤 작품의 몇 화를 쓰고 있는지 바로 보이고, 다른 화수로 이동하기도 용이합니다. 전 여러 작품을 동시에 볼 때가 많은데 파일을 일일이 백업할 필요가 없어서 좋았어요.',
      url: 'https://x.com/purpledora2/status/1937485680575434818',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1907472029370167296/vHvEpnCv_400x400.jpg',
      author: '레슈',
      handle: '@skydowny',
      content:
        '타이피 좋아요...\n다른 것도 다 좋은데 매일 얼마나 썼는지 기록이 되는 게 제일 좋음... 글 열심히 쓸 수 있게 동기부여가 되어 주는 것 같아요 오늘 날짜에 색칠하려고 조금이라도 쓰게 돼서 참 조음\n덕분에 글 샥샥이 기계가 됐다네요 굿 타이피',
      url: 'https://x.com/skydowny/status/1936639292354547957',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1909293418960031744/2J4lqgcX_400x400.jpg',
      author: '초재벌갑부박쥐',
      handle: '@sugobat',
      content:
        '언젠가 이런 서비스가 오리라 믿고 있었습니다.\n진짜 디테일하게 여러 기능들이 많고요\n써보면서 개인적으로 마음에 들었던 것들 두서 없이 적어보겠습니다\n\n0. 데스크탑/모바일 UI가 통일되어있음\n1. 지금까지 쓴 글자수 확인 가능 (전일/특정 달 글자수 비교 가능)\n2. ★폰트를 직접 추가 가능★',
      url: 'https://x.com/sugobat/status/1937516814310989972',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1907316934905131008/DmV5HFLn_400x400.jpg',
      author: '마눌고양이',
      handle: '@Giantsquidbig',
      content: '타이피 정말 글쓰기 좋은 툴임',
      url: 'https://x.com/Giantsquidbig/status/1940645747948769482',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1729053424979951617/8tloydYo_400x400.jpg',
      author: '보미',
      handle: '@bombom_mabi',
      content: '타이피 정말 맘에 들어. 갈수록 맘에 들어.',
      url: 'https://x.com/bombom_mabi/status/1943148481918701846',
    },
  ];
  // spell-checker:enable

  const makeColumns = (items: Testimonial[]) => {
    const columns = [[], [], []] as Testimonial[][];
    items.forEach((item, index) => {
      columns[index % 3].push(item);
    });
    return columns;
  };

  const columns = makeColumns(testimonials);
</script>

<section class={css({ position: 'relative', paddingY: { sm: '80px', lg: '120px' }, backgroundColor: 'gray.50' })}>
  <div class={css({ position: 'relative', maxWidth: '[1024px]', marginX: 'auto', paddingX: { sm: '16px', lg: '40px' } })}>
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
          paddingX: { sm: '16px', lg: '20px' },
          paddingY: { sm: '6px', lg: '8px' },
          marginBottom: { sm: '24px', lg: '40px' },
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: { sm: '12px', lg: '14px' },
          fontWeight: 'bold',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: { sm: 'rotate(0)', lg: 'rotate(-2deg)' },
        })}
      >
        TESTIMONIALS
      </div>

      <h2
        class={css({
          fontSize: { sm: '[36px]', lg: '[56px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: { sm: '16px', lg: '24px' },
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        실제 이용자가 전하는
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: { sm: '16px', lg: '20px' },
            display: 'inline-block',
            transform: { sm: 'rotate(0)', lg: 'rotate(1deg)' },
          })}
        >
          생생한 후기
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
        실제로 써본 사람들이 말하는 타이피의 장점, 지금 직접 확인해보세요.
      </p>
    </div>

    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: 'repeat(3, 1fr)' },
        gap: { sm: '16px', lg: '24px' },
        alignItems: 'start',
      })}
    >
      {#each columns as column, colIndex (colIndex)}
        <div
          style:transition={`opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) ${0.1 + colIndex * 0.1}s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) ${0.1 + colIndex * 0.1}s`}
          class={css({
            display: 'flex',
            flexDirection: 'column',
            gap: { sm: '16px', lg: '24px' },
            opacity: '0',
            transform: 'translate3d(0, 20px, 0)',
            willChange: 'transform, opacity',
            '&.in-view': {
              opacity: '100',
              transform: 'translate3d(0, 0, 0)',
            },
          })}
          data-observe
        >
          {#each column as testimonial, idx (idx)}
            <a
              class={css({
                display: 'block',
                padding: { sm: '16px', lg: '24px' },
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                textDecoration: 'none',
                transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                boxShadow: '[6px 6px 0 0 #000]',
                transform: { sm: 'translate3d(0, 0, 0)', lg: idx % 2 === 0 ? 'rotate(-1deg)' : 'rotate(1deg)' },
                _hover: {
                  transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0deg)' },
                  boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[10px 10px 0 0 #000]' },
                },
              })}
              href={testimonial.url}
              rel="noopener noreferrer"
              target="_blank"
            >
              <div class={flex({ alignItems: 'center', gap: { sm: '8px', lg: '12px' }, marginBottom: { sm: '12px', lg: '16px' } })}>
                <img
                  class={css({
                    size: { sm: '32px', lg: '40px' },
                    backgroundColor: 'gray.200',
                    objectFit: 'cover',
                    border: '3px solid',
                    borderColor: 'gray.900',
                  })}
                  alt={testimonial.author}
                  src={testimonial.avatar}
                />
                <div class={css({ flex: '1' })}>
                  <div
                    class={css({
                      fontSize: { sm: '14px', lg: '16px' },
                      fontWeight: 'black',
                      color: 'gray.900',
                      textTransform: 'uppercase',
                    })}
                  >
                    {testimonial.author}
                  </div>
                  <div
                    class={css({
                      fontSize: { sm: '12px', lg: '13px' },
                      fontWeight: 'medium',
                      color: 'gray.600',
                    })}
                  >
                    {testimonial.handle}
                  </div>
                </div>
              </div>

              <p
                class={css({
                  fontSize: { sm: '14px', lg: '15px' },
                  lineHeight: '[1.7]',
                  color: 'gray.800',
                  whiteSpace: 'pre-wrap',
                  fontWeight: 'medium',
                })}
              >
                {testimonial.content}
              </p>
            </a>
          {/each}
        </div>
      {/each}
    </div>
  </div>
</section>
