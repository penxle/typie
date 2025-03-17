<script lang="ts">
  import AgeRestriction from '$assets/graphics/age-restriction.svg?component';
  import PrivatePost from '$assets/graphics/private-post.svg?component';
  import ShareLink from '$assets/graphics/share-link.svg?component';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';

  const cards = [
    {
      title: '링크 공유',
      description: '링크를 생성해 원하는 독자에게만\n나의 창작물을 공유할 수 있어요.',
      asset: ShareLink,
    },
    {
      title: '비밀글 설정',
      description: '비밀번호가 설정된 창작물은 오직\n비밀번호를 아는 독자만 볼 수 있어요.',
      asset: PrivatePost,
    },
    {
      title: '연령 제한',
      description: '창작물 별로 연령 제한을 설정하면\n안전하게 콘텐츠를 제공할 수 있어요.',
      asset: AgeRestriction,
    },
  ];

  const cardStyle = css.raw({
    borderRadius: '16px',
    paddingTop: '30px',
    paddingX: '36px',
    paddingBottom: '40px',
    boxShadow: '[0px 4px 8px 2px rgba(0, 0, 0, 0.08)]',
    textAlign: 'left',
    width: 'full',
    '& > div': {
      marginBottom: '40px',
      fontSize: '28px',
      fontWeight: '[700]',
    },
    '& > p': {
      fontSize: '14px',
      lineHeight: '[1.8]',
    },
  });
</script>

<div class={flex({ direction: 'column', align: 'center', marginY: '161px' })}>
  <div class={css({ textAlign: 'center', width: 'full', maxWidth: '900px' })}>
    <h2 class={css({ marginBottom: '30px', fontFamily: '[LINESeedKR]', fontSize: '[42px]', fontWeight: '[700]' })}>
      원하는 순간, 원하는 독자에게만
    </h2>

    <p>
      링크를 생성하거나 비밀번호를 설정해서 원하는 독자에게만 창작물을 공유할 수 있어요.
      <br />
      전하고 싶은 만큼만, 원하는 사람에게만 안전하게 공유하세요.
    </p>

    <div class={grid({ columns: 3, gap: '30px', marginTop: '60px', marginBottom: '30px' })}>
      {#each cards as { title, description, asset } (title)}
        <div
          class={flex({
            direction: 'column',
            align: 'center',
            justify: 'center',
            gap: '30px',
            paddingX: '30px',
            paddingY: '50px',
            backgroundColor: '[#F9FAFB]',
            boxShadow: '[0px 8px 12px 0px rgba(0, 0, 0, 0.03), 0px 12px 24px 0px rgba(0, 0, 0, 0.03)]',
          })}
        >
          <svelte:component this={asset} />

          <div>
            <p class={css({ marginBottom: '4px', fontWeight: '[700]' })}>{title}</p>
            <p class={css({ fontSize: '14px' })}>
              {description}
            </p>
          </div>
        </div>
      {/each}
    </div>

    <div class={flex({ gap: '30px' })}>
      <div class={css({ backgroundColor: '[#B1D8CB]' }, cardStyle)}>
        <div>
          공개 할 부분과
          <br />
          공개 기간까지 내 마음대로
        </div>

        <p>
          소중한 내 창작물, 가끔은 몇 명의 사람에게 조금만 보여주고 <br />
          싶을 때가 있죠. 링크 공유와 비밀글 설정으로 정해진 기간 동안,
          <br />
          한정된 독자에게만 공유해보세요.
        </p>
      </div>

      <div class={css({ backgroundColor: '[#F7DE9C]' }, cardStyle)}>
        <div>
          한정된 독자에게만 닿는
          <br />
          세심한 공유 방식
        </div>

        <p>
          어떨 땐 모든 연령에게, 때로는 15세 이상, 가끔은 성인에게만!
          <br />
          연령 제한을 설정하면 독자들이 안전하게 콘텐츠를 즐길 수 있어요.
        </p>
      </div>
    </div>
  </div>
</div>
