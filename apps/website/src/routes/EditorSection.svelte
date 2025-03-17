<script lang="ts">
  import Blocks from '$assets/graphics/blocks.svg?component';
  import BubbleMenu from '$assets/graphics/bubble-menu.svg?component';
  import SpellCheck from '$assets/graphics/spell-check.svg?component';
  import SynonymSuggestion from '$assets/graphics/synonym-suggestion.svg?component';
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
      description: '국립국어원의 기준에 맞는 맞춤법과 문법을\n기반으로 완성도 높은 글을 쓸 수 있어요.',
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
      description: '구분선, 인용구, 표 등의 다양한 구성 요소를\n글에 삽입해서 풍성한 글을 만들 수 있어요.',
      cardStyle: css.raw({ backgroundColor: '[#F3F3F3]' }),
      asset: Blocks,
    },
  ];
</script>

<div class={flex({ direction: 'column', align: 'center', gap: '60px', marginTop: '190px', paddingBottom: '82px' })}>
  <div class={css({ textAlign: 'center' })}>
    <h2 class={css({ marginBottom: '30px', fontSize: '[42px]', fontWeight: '[700]' })}>틀에 갇히지 않는 자유로운 창작</h2>

    <p>
      기본적인 텍스트 편집은 물론, 자간, 행간 조절, 들여쓰기 등 세밀한 옵션으로
      <br />
      작품을 나만의 방식대로 자유롭게 표현할 수 있습니다.
    </p>
  </div>

  <div class={grid({ columns: 2, gap: '20px', paddingX: '25px', width: 'full', maxWidth: '900px' })}>
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
</div>
