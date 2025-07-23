<script lang="ts">
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

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
    // {
    //   avatar: 'https://pbs.twimg.com/profile_images/1947225921330483200/2HCsb7RP_400x400.jpg',
    //   author: '정오',
    //   handle: '@1114effect',
    //   content:
    //     '타이피 잘 쓰고 있습니다 ^.^ 회지 내야할 행사가 연달아 있어서 매일매일 원고 작업하고 있는데, 모바일로도 나와준 덕분에 집이 아닌 다른 곳에서도 편하게 원고 하고 있어요 ㅎ.ㅎ 앞으로도 승승장구 하시고 오래오래 운영해주시길 바랍니다...…',
    //   url: 'https://x.com/1114effect/status/1936438693952397378',
    // },
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
    // {
    //   avatar: 'https://pbs.twimg.com/profile_images/1947870724129230848/TwpuCZC2_400x400.jpg',
    //   author: '은하',
    //   handle: '@Eunha_0847D',
    //   content:
    //     '진심 글 쓸 때마다 밑에 메모장 켜놓고 메모장용 한글 켜놓고 워드 켜놓고 그거 올렸다 내렸다 어디있었는지 까먹고 딸깍딸깍 난리를 친 사람으로 타이피 쓰고 창 완전 깔끔해졌음 그냥 타이피만 쓰면 됨',
    //   url: 'https://x.com/Eunha_0847D/status/1937817913509212169',
    // },
    // {
    //   avatar: 'https://pbs.twimg.com/profile_images/1946066127345352713/HoIGDiq7_400x400.jpg',
    //   author: '제거사',
    //   handle: '@SPTHX0516',
    //   content:
    //     '요즘 타이피 결제해서 글 쓰고 있는데 너무 좋아요...비번 걸어서 공유하는 기능 << 드림하기에 너무 킥이고!!! 글쓰기에 이만큼 최적화된 사이트가 없다보니 콘티 정리나 창작에 유용하게 사용합니다!!! 창작/드림은 무조건 이쪽으로 공유할 것 같아요...^^',
    //   url: 'https://x.com/SPTHX0516/status/1940294565371813927',
    // },
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
        transform: { sm: 'translateY(20px)', lg: 'translateY(20px) rotate(-1deg)' },
        transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        '&.in-view': {
          opacity: '100',
          transform: { sm: 'translateY(0)', lg: 'translateY(0) rotate(0)' },
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
          transform: 'rotate(-2deg)',
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
            transform: 'rotate(1deg)',
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
            transform: 'translateY(20px)',
            '&.in-view': {
              opacity: '100',
              transform: 'translateY(0)',
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
                transform: { sm: 'rotate(0)', lg: idx % 2 === 0 ? 'rotate(-1deg)' : 'rotate(1deg)' },
                _hover: {
                  transform: { sm: 'translate(-4px, -4px)', lg: 'translate(-4px, -4px) rotate(0deg)' },
                  boxShadow: '[10px 10px 0 0 #000]',
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
