<script lang="ts">
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
</script>

<section class={css({ position: 'relative', paddingY: '120px', backgroundColor: 'gray.50' })}>
  <div class={css({ position: 'relative', maxWidth: '[1024px]', marginX: 'auto' })}>
    <div class={center({ flexDirection: 'column', marginBottom: '80px' })}>
      <h2
        class={css({
          fontSize: '[44px]',
          fontWeight: 'extrabold',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '16px',
          lineHeight: '[1.2]',
        })}
      >
        먼저 사용해 본 사람들의 이야기
      </h2>
      <p
        class={css({
          fontSize: '18px',
          fontWeight: 'medium',
          color: 'gray.600',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '600px',
          lineHeight: '[1.6]',
        })}
      >
        다양한 분야의 사용자들이 어떻게 타이피를 활용하고 있는지 확인해 보세요.
      </p>
    </div>

    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: '20px',
        alignItems: 'start',
      })}
    >
      {#each columns as column, colIndex (colIndex)}
        <div class={css({ display: 'flex', flexDirection: 'column', gap: '20px' })}>
          {#each column as testimonial, idx (idx)}
            <a
              class={css({
                display: 'block',
                padding: '20px',
                borderRadius: '16px',
                backgroundColor: 'white',
                borderWidth: '1px',
                borderColor: 'gray.200',
                cursor: 'pointer',
                textDecoration: 'none',
                transition: 'common',
                _hover: {
                  borderColor: 'amber.500',
                  transform: 'translateY(-2px)',
                  boxShadow: '[0 4px 12px rgba(0, 0, 0, 0.08)]',
                },
              })}
              href={testimonial.href}
              rel="noopener noreferrer"
              target="_blank"
            >
              <div class={flex({ alignItems: 'center', gap: '12px', marginBottom: '12px' })}>
                <img
                  class={css({
                    size: '40px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.200',
                    objectFit: 'cover',
                  })}
                  alt={testimonial.author}
                  src={testimonial.avatar}
                />
                <div class={css({ flex: '1' })}>
                  <span
                    class={css({
                      fontSize: '15px',
                      fontWeight: 'bold',
                      color: 'gray.900',
                      fontFamily: 'Pretendard',
                    })}
                  >
                    {testimonial.author}
                  </span>
                </div>
              </div>

              <p
                class={css({
                  fontSize: '15px',
                  lineHeight: '[1.6]',
                  color: 'gray.900',
                  fontFamily: 'Pretendard',
                  whiteSpace: 'pre-wrap',
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

  <div
    class={css({
      position: 'absolute',
      bottom: '120px',
      left: '0',
      right: '0',
      height: '200px',
      background: '[linear-gradient(transparent, rgb(249, 250, 251))]',
      pointerEvents: 'none',
    })}
  ></div>
</section>
