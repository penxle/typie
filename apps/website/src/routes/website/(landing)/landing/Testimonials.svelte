<script lang="ts">
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  type Testimonial = {
    content: string;
    author: string;
    avatar: string;
  };

  const testimonials: Testimonial[] = [
    {
      content: '타이피 쓰면서 제일 좋은 점 - 모바일 앱도 있어서 갑자기 아이디어 떠오르면 휴대폰으로 바로 작성할 수 있음',
      author: '현준',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=hyunjun',
    },
    {
      content:
        '웹소설 작가들 타이피 안 써봤으면 진짜 추천함\n폴더로 에피소드별 정리 가능하고 캐릭터 설정이나 관계도 따로 만들어서 관리하기 편함. 특히 내용이 실시간으로 저장돼서 날린 적이 없어서 좋음',
      author: '준호',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=junho',
    },
    {
      content: '동시편집으로 담당 PD랑 실시간으로 같이 글 보면서 수정할 수 있는 게 진짜 편함. 웹소설 작가한테는 정말 유용한 기능인 듯',
      author: '지민',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=jimin',
    },
    {
      content: '타이피 앵커 기능 너무 좋다..\n앵커 등록해놓으면 글 정리할 때 목차별로 바로바로 이동 가능해서 스크롤 지옥에서 해방됨',
      author: '유진',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=yujin',
    },
    {
      content:
        '블로그 포스팅 전에 타이피에서 초안 작성하고 퇴고하는 중\n여러 버전 저장해두고 비교하면서 수정할 수 있어서 글쓰기가 훨씬 편해짐!',
      author: '하늘',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=haneul',
    },
    {
      content:
        '글이나 폴더 단위로 링크 공유할 수 있어서 보여주고 싶은 사람한테만 공유하기 좋음! 비밀번호나 연령 제한도 설정할 수 있어서 진짜 유용함',
      author: '민아',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=mina',
    },
    {
      content: '타이피에는 글쓰기 기록이 있어서 내가 매일 얼마나 썼는지 바로 확인할 수 있는 게 너무 좋음',
      author: '서연',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=seoyeon',
    },
    {
      content: 'UI도 예쁘고 광고가 없으니까 정말 글쓰기에만 집중할 수 있어서 좋음',
      author: '수빈',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=subin',
    },
    {
      content: '요즘은 타이피 캔버스에 플롯이나 인물 관계도를 정리해두는데, 이렇게 하니까 전체 맥락이 보이면서 방향 잡기가 쉬워짐',
      author: '태현',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=taehyun',
    },
  ];

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
          marginBottom: { sm: '24px', lg: '32px' },
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: { sm: '12px', lg: '14px' },
          fontWeight: 'bold',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: 'rotate(-2deg)',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[4px 4px 0 0 #fbbf24]',
        })}
      >
        TESTIMONIALS
      </div>

      <h2
        class={css({
          fontSize: { sm: '[36px]', lg: '[56px]' },
          fontWeight: 'black',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: { sm: '16px', lg: '24px' },
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: { sm: '16px', lg: '20px' },
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          실제 이용자가
        </span>
        <br />
        말하는 타이피
      </h2>
      <p
        class={css({
          fontSize: { sm: '16px', lg: '20px' },
          fontWeight: 'semibold',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '600px',
          lineHeight: '[1.7]',
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
            <div
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
                  <span
                    class={css({
                      fontSize: { sm: '14px', lg: '16px' },
                      fontWeight: 'black',
                      color: 'gray.900',
                      fontFamily: 'Pretendard',
                      textTransform: 'uppercase',
                    })}
                  >
                    {testimonial.author}
                  </span>
                </div>
              </div>

              <p
                class={css({
                  fontSize: { sm: '14px', lg: '15px' },
                  lineHeight: '[1.7]',
                  color: 'gray.800',
                  fontFamily: 'Pretendard',
                  whiteSpace: 'pre-wrap',
                  fontWeight: 'medium',
                })}
              >
                {testimonial.content}
              </p>
            </div>
          {/each}
        </div>
      {/each}
    </div>
  </div>
</section>
