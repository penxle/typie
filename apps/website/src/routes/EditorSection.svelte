<script lang="ts">
  import Blocks from '$assets/graphics/blocks.svg?component';
  import BubbleMenu from '$assets/graphics/bubble-menu.svg?component';
  import Decoration from '$assets/graphics/decoration1.svg?component';
  import SpellCheck from '$assets/graphics/spell-check.svg?component';
  import SynonymSuggestion from '$assets/graphics/synonym-suggestion.svg?component';
  import Underline from '$assets/graphics/underline.svg?component';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';

  const cards = [
    {
      title: '자유로운 편집 디자인',
      description: '세밀한 자간, 행간 조절과 들여쓰기 등 자유로운 편집 옵션을 제공해요.',
      cardStyle: css.raw({ alignItems: 'flex-start', backgroundColor: '[#F3F3F3]' }),
      asset: BubbleMenu,
    },
    {
      title: '맞춤법 검사',
      description: '클릭 한 번으로 맞춤법과 문법을 검사해\n글의 완성도를 높일 수 있어요.',
      cardStyle: css.raw({ backgroundColor: '[#FFFAED]' }),
      asset: SpellCheck,
    },
    {
      title: '동의어 추천',
      description: '다양한 표현이 담긴 문장을 원한다면\n스마트한 동의어 추천을 활용해보세요.',
      cardStyle: css.raw({ backgroundColor: '[#EBF2EF]' }),
      asset: SynonymSuggestion,
    },
    {
      title: '다양한 꾸밈 기능',
      description: '구분선, 인용구, 표 등의 다양한 구성 요소를\n글에 삽입해 풍성한 글을 만들 수 있어요.',
      cardStyle: css.raw({ backgroundColor: '[#F3F3F3]' }),
      asset: Blocks,
    },
  ];
</script>

<div class={flex({ direction: 'column', align: 'center', gap: '60px', marginTop: '210px', paddingBottom: '82px' })}>
  <div class={css({ textAlign: 'center' })}>
    <div class={css({ position: 'relative' })}>
      <h2
        class={css({
          position: 'relative',
          marginBottom: '30px',
          fontFamily: '[LINESeedKR]',
          fontSize: '[42px]',
          fontWeight: '[700]',
          zIndex: '1',
        })}
      >
        창작의 결을 그대로 살리는 에디터
      </h2>
      <Underline class={css({ position: 'absolute', bottom: '-6px', left: '0', zIndex: '0', color: '[#B1ACE2]' })} />
    </div>

    <p>
      기본적인 텍스트 편집은 물론, 다양한 꾸밈 요소와 글쓰기 편의 기능으로
      <br />
      작품의 완성도를 높이고 나만의 개성을 더할 수 있어요.
    </p>
  </div>

  <div class={css({ position: 'relative', width: 'full', maxWidth: '900px' })}>
    <div class={grid({ columns: 2, gap: '20px', paddingX: '25px' })}>
      {#each cards as { title, description, cardStyle, asset } (title)}
        <div
          class={css(
            { display: 'flex', alignItems: 'flex-end', flexDirection: 'column', borderRadius: '[17px]', padding: '20px', width: 'full' },
            cardStyle,
          )}
        >
          <div class={css({ whiteSpace: 'pre-line', width: 'full' })}>
            <h3 class={css({ marginBottom: '12px', fontWeight: '[600]' })}>{title}</h3>

            <p>{description}</p>
          </div>

          <svelte:component this={asset} class={css({ marginTop: '50px' })} />
        </div>
      {/each}
    </div>

    <Decoration
      class={css({
        position: 'absolute',
        translate: 'auto',
        translateY: '-1/2',
        top: '1/2',
        right: '-116px',
        zIndex: '0',
        color: '[#F7DE9C]',
      })}
    />
  </div>
</div>
