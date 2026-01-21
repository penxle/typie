<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { cubicOut } from 'svelte/easing';
  import { slide } from 'svelte/transition';
  import { inview } from './inview';

  let expandedIds = $state<Set<string>>(new Set());

  const toggle = (id: string) => {
    expandedIds = new Set(expandedIds.has(id) ? [...expandedIds].filter((i) => i !== id) : [...expandedIds, id]);
  };

  const isExpanded = (id: string) => expandedIds.has(id);

  const isAllExpanded = $derived(expandedIds.size === 13);

  const toggleAll = () => {
    if (isAllExpanded) {
      expandedIds = new Set();
    } else {
      expandedIds = new Set(['01', '02', '03', '04', '05', '06', '07', '08', '09', '10', '11', '12', '13']);
    }
  };

  const features = [
    {
      id: '01',
      title: '자동 저장',
      summary: '쓰는 족족 저장, 날아갈 걱정 없이',
      description: '모든 글이 실시간으로 저장돼요.\n창을 닫거나 연결이 끊겨도 마지막 상태 그대로 돌아와요.',
    },
    {
      id: '02',
      title: '집중 모드',
      summary: '흐트러지지 않는 시선',
      description: '타자기 모드로 현재 줄을 화면 중앙에 고정하고, 시선 유도로 작성 중인 문장만 강조해요.\n눈과 손이 자연스럽게 따라가요.',
    },
    {
      id: '03',
      title: '테마',
      summary: '라이트, 다크 모드 16가지 색상',
      description: '라이트와 다크 모드 각각 8가지 색상 테마를 제공해요.\n눈의 피로를 줄이고 취향에 맞게 글쓰기 환경을 꾸며보세요.',
    },
    {
      id: '04',
      title: '노트',
      summary: '떠오르는 아이디어를 바로 메모',
      description: '글쓰기에 도움이 되는 내용을 자유롭게 적어두세요.\n관련 원고에 연결해두면 나중에 쉽게 찾을 수 있어요.',
    },
    {
      id: '05',
      title: '공유',
      summary: '링크 하나로 공유, 실시간 협업',
      description: '글을 공유하고 동시에 편집할 수 있어요.\n피드백 요청, 감수, 공동 작업이 훨씬 간단해져요.',
    },
    {
      id: '06',
      title: '작성 기록',
      summary: '매일 얼마나 썼는지 한눈에',
      description: '글쓰기 기록이 자동으로 쌓여요.\n꾸준히 쓰는 리듬을 만들고, 달력을 하나씩 채워가세요.',
    },
    {
      id: '07',
      title: '타임라인',
      summary: '버전 관리, 원하는 시점으로 복원',
      description: '글의 모든 변경 이력이 자동으로 저장돼요.\n슬라이더로 과거 버전을 탐색하고 원하는 시점으로 복원하세요.',
    },
    {
      id: '08',
      title: '크로스 플랫폼',
      summary: '웹, iOS, Android 어디서든 이어쓰기',
      description: '기기를 바꿔도 작업이 실시간으로 동기화돼요.\n집에서 쓰다가 밖에서 떠오른 문장을 바로 이어붙이세요.',
    },
    {
      id: '09',
      title: '서식',
      summary: '문단 간격, 폰트, 본문 폭까지',
      description: '원고 서식을 세밀하게 조정할 수 있어요.\n원하는 폰트가 없다면 직접 업로드해서 쓰세요.',
    },
    {
      id: '10',
      title: '템플릿',
      summary: '반복되는 포맷을 저장해두기',
      description: '자주 쓰는 포맷을 템플릿으로 저장해두세요.\n새 글을 시작할 때마다 빈 페이지에서 고민하지 않아도 돼요.',
    },
    {
      id: '11',
      title: '위젯',
      summary: '글자 수, 타이머 등 글쓰기 보조 도구',
      description: '에디터 옆에 필요한 위젯을 자유롭게 배치하세요.\n글자 수, 오늘의 기록, 타이머 등 다양한 도구가 글쓰기를 도와요.',
    },
    {
      id: '12',
      title: '유료 블록',
      summary: '원하는 부분만 유료로 설정',
      description: '포스트 안에 유료 콘텐츠 영역을 만들 수 있어요.\n독자가 포인트로 구매해서 열람할 수 있어요.',
    },
    {
      id: '13',
      title: '검색',
      summary: '필요한 글과 문장을 바로 찾기',
      description: '단축키로 빠르게 검색하고, 앵커로 자주 가는 위치를 표시해두세요.\n긴 글 안에서도 원하는 곳으로 바로 이동할 수 있어요.',
    },
  ];
</script>

<section
  class={css({
    position: 'relative',
    paddingX: { sm: '24px', lg: '80px' },
    paddingY: { sm: '80px', lg: '120px' },
    backgroundColor: 'dark.gray.950',
  })}
>
  <div
    class={css({
      position: 'absolute',
      left: { sm: '16px', lg: '48px' },
      top: '0',
      bottom: '0',
      width: '1px',
      backgroundColor: 'dark.gray.800',
      display: { sm: 'none', lg: 'block' },
    })}
  ></div>

  <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
    <div
      class={css({
        marginBottom: { sm: '48px', lg: '80px' },
        opacity: '0',
        transform: 'translate3d(0, 28px, 0)',
        transition: '[opacity 0.8s cubic-bezier(0.16, 1, 0.3, 1), transform 0.8s cubic-bezier(0.16, 1, 0.3, 1)]',
        '&.in-view': {
          opacity: '100',
          transform: 'translate3d(0, 0, 0)',
        },
      })}
      {@attach inview}
    >
      <span
        class={css({
          display: 'block',
          fontSize: '[11px]',
          fontFamily: 'mono',
          color: 'dark.gray.500',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          marginBottom: '24px',
        })}
      >
        Features
      </span>

      <div class={css({ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', gap: '24px' })}>
        <h2
          class={css({
            fontSize: { sm: '[32px]', lg: '[48px]' },
            fontWeight: 'medium',
            color: 'dark.gray.100',
            lineHeight: '[1.2]',
            letterSpacing: '[-0.02em]',
            fontFamily: 'Paperlogy',
          })}
        >
          작가가 원하는 것,
          <br />
          <span class={css({ color: 'dark.gray.400' })}>그대로.</span>
        </h2>

        <button
          class={css({
            fontSize: '[24px]',
            fontFamily: 'mono',
            fontWeight: 'medium',
            color: 'dark.gray.600',
            background: '[none]',
            border: '[none]',
            cursor: 'pointer',
            padding: '0',
            flexShrink: '[0]',
            transition: '[color 0.2s]',
            _hover: { color: 'dark.gray.400' },
          })}
          onclick={toggleAll}
          type="button"
        >
          {isAllExpanded ? '−' : '+'}
        </button>
      </div>
    </div>

    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: 'repeat(4, 1fr)' },
        gridTemplateRows: { lg: 'repeat(4, auto)' },
        gridAutoFlow: { lg: 'column' },
        gap: { sm: '32px', lg: '[56px 32px]' },
        opacity: '0',
        transform: 'translate3d(0, 20px, 0)',
        transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
        '&.in-view': {
          opacity: '100',
          transform: 'translate3d(0, 0, 0)',
        },
      })}
      {@attach inview}
    >
      {#each features as feature (feature.id)}
        <button
          class={css({
            display: 'flex',
            alignItems: 'flex-start',
            gap: { sm: '16px', lg: '20px' },
            width: '[100%]',
            textAlign: 'left',
            cursor: 'pointer',
            background: '[none]',
            border: '[none]',
            padding: '0',
            minHeight: '[88px]',
          })}
          onclick={() => toggle(feature.id)}
          type="button"
        >
          <span
            class={css({
              fontSize: '[28px]',
              fontFamily: 'mono',
              fontWeight: 'light',
              color: isExpanded(feature.id) ? 'dark.brand.400' : 'dark.gray.700',
              flexShrink: '[0]',
              lineHeight: '[1]',
              transition: '[color 0.2s]',
            })}
          >
            {feature.id}
          </span>

          <div class={css({ flex: '1', minWidth: '0' })}>
            <div class={css({ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '6px' })}>
              <h3
                class={css({
                  fontSize: { sm: '18px', lg: '20px' },
                  fontWeight: 'medium',
                  color: 'dark.gray.100',
                  fontFamily: 'Paperlogy',
                })}
              >
                {feature.title}
              </h3>
              <span
                class={css({
                  fontSize: '[20px]',
                  fontFamily: 'mono',
                  fontWeight: 'medium',
                  color: 'dark.gray.600',
                })}
              >
                {isExpanded(feature.id) ? '−' : '+'}
              </span>
            </div>

            <p
              class={css({
                fontSize: { sm: '14px', lg: '15px' },
                color: 'dark.gray.300',
                lineHeight: '[1.55]',
              })}
            >
              {feature.summary}
            </p>

            {#if isExpanded(feature.id)}
              <p
                class={css({
                  fontSize: { sm: '13px', lg: '14px' },
                  color: 'dark.gray.500',
                  lineHeight: '[1.65]',
                  whiteSpace: 'normal',
                  marginTop: '12px',
                  height: { lg: '[100px]' },
                })}
                transition:slide={{ duration: 250, easing: cubicOut }}
              >
                {feature.description}
              </p>
            {/if}
          </div>
        </button>
      {/each}
    </div>
  </div>
</section>
