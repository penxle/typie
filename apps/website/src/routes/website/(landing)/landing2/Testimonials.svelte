<script lang="ts">
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  type Testimonial = {
    content: string;
    author: string;
    avatar: string;
    href: string;
  };

  const testimonials: Testimonial[] = [
    {
      content:
        '타이피로 일기 쓴지 100일 돌파! 🎉 진짜 캔버스 기능이 대박인게 그날 기분을 그림으로도 표현할 수 있어서 너무 좋음... 나중에 다시 보면 그날이 바로 떠올라',
      author: '서연',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=seoyeon',
      href: 'https://typie.app/@seoyeon',
    },
    {
      content:
        '웹소설 작가들 타이피 안 써봤으면 진짜 추천\n\n폴더로 에피소드별 정리 가능하고 캐릭터 설정이나 세계관 문서 따로 만들어서 관리하기 편함ㅋㅋㅋ 특히 실시간 저장 기능 때문에 날린 적이 없어서 좋음',
      author: '준호',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=junho',
      href: 'https://typie.app/@junho',
    },
    {
      content:
        '팀 미팅하면서 실시간으로 같이 문서 작성하는 거 진짜 편하다... 화면 공유 안 해도 되고 바로바로 수정사항 반영되니까 회의 시간이 절반으로 줄었음',
      author: '지민',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=jimin',
      href: 'https://typie.app/@jimin',
    },
    {
      content: '아니 타이피 단락 앵커 기능 미쳤네;;;; 논문 정리할 때 목차별로 바로바로 이동 가능해서 스크롤 지옥에서 해방됨ㅠㅠ',
      author: '유진',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=yujin',
      href: 'https://typie.app/@yujin',
    },
    {
      content:
        '블로그 포스팅 전에 타이피에서 초안 작성하고 퇴고하는 중\n\n여러 버전 저장해두고 비교하면서 수정할 수 있어서 글쓰기가 훨씬 편해짐! 특히 마크다운 지원해서 바로 복붙 가능 👍',
      author: '하늘',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=haneul',
      href: 'https://typie.app/@haneul',
    },
    {
      content:
        '여행 다녀온 거 타이피에 정리했는데 진짜 만족스러움... 사진이랑 같이 그날의 감정까지 적어두니까 나중에 봐도 그때 기분이 고스란히 전해져서 좋아요 🥹',
      author: '민아',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=mina',
      href: 'https://typie.app/@mina',
    },
    {
      content:
        '타이피 쓰면서 제일 좋은 점: 글쓰다가 갑자기 아이디어 떠오르면 캔버스에 바로 그려서 시각화할 수 있음. 텍스트로만 생각 정리하는 것보다 훨씬 효과적',
      author: '현준',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=hyunjun',
      href: 'https://typie.app/@hyunjun',
    },
    {
      content: '대학 과제할 때 타이피 없었으면 어떻게 했을까 싶음... 조별과제 할 때 실시간 협업 기능 진짜 꿀',
      author: '수빈',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=subin',
      href: 'https://typie.app/@subin',
    },
    {
      content: '타이피에서 하루 회고 쓰는 게 일상이 됐는데, 한 달 지나고 보니까 내가 어떻게 성장했는지 한눈에 보여서 뿌듯함 ㅎㅎ',
      author: '태현',
      avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=taehyun',
      href: 'https://typie.app/@taehyun',
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

  let headerElement = $state<HTMLElement>();
  let columnElements = $state<HTMLElement[]>([]);

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

    if (headerElement) observer.observe(headerElement);
    columnElements.forEach((element) => {
      if (element) observer.observe(element);
    });

    return () => {
      if (headerElement) observer.unobserve(headerElement);
      columnElements.forEach((element) => {
        if (element) observer.unobserve(element);
      });
    };
  });
</script>

<section class={css({ position: 'relative', paddingY: '120px', backgroundColor: 'gray.50' })}>
  <div class={css({ position: 'relative', maxWidth: '[1024px]', marginX: 'auto', paddingX: '40px' })}>
    <div
      bind:this={headerElement}
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
          paddingX: '20px',
          paddingY: '8px',
          marginBottom: '32px',
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: '14px',
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
          fontSize: '[56px]',
          fontWeight: 'black',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '24px',
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        먼저 사용해 본
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          사람들의 이야기
        </span>
      </h2>
      <p
        class={css({
          fontSize: '20px',
          fontWeight: 'semibold',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '600px',
          lineHeight: '[1.7]',
        })}
      >
        다양한 분야의 사용자들이 어떻게 타이피를 활용하고 있는지 확인해 보세요.
      </p>
    </div>

    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: '24px',
        alignItems: 'start',
      })}
    >
      {#each columns as column, colIndex (colIndex)}
        <div
          bind:this={columnElements[colIndex]}
          style:transition={`opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) ${0.1 + colIndex * 0.1}s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) ${0.1 + colIndex * 0.1}s`}
          class={css({
            display: 'flex',
            flexDirection: 'column',
            gap: '24px',
            opacity: '0',
            transform: 'translateY(20px)',
            '&.in-view': {
              opacity: '100',
              transform: 'translateY(0)',
            },
          })}
        >
          {#each column as testimonial, idx (idx)}
            <a
              class={css({
                display: 'block',
                padding: '24px',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                cursor: 'pointer',
                textDecoration: 'none',
                transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                boxShadow: '[6px 6px 0 0 #000]',
                transform: idx % 2 === 0 ? 'rotate(-1deg)' : 'rotate(1deg)',
                _hover: {
                  transform: 'translate(-4px, -4px) rotate(0deg)',
                  boxShadow: '[10px 10px 0 0 #000]',
                },
              })}
              href={testimonial.href}
              rel="noopener noreferrer"
              target="_blank"
            >
              <div class={flex({ alignItems: 'center', gap: '12px', marginBottom: '16px' })}>
                <img
                  class={css({
                    size: '40px',
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
                      fontSize: '16px',
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
                  fontSize: '15px',
                  lineHeight: '[1.7]',
                  color: 'gray.800',
                  fontFamily: 'Pretendard',
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
