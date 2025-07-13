<script lang="ts">
  import GithubIcon from '~icons/simple-icons/github';
  import TwitterIcon from '~icons/simple-icons/twitter';
  import Wordmark from '$assets/logos/wordmark.svg?component';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div class={css({ backgroundColor: 'gray.50' })}>
  <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'dark.gray.950' })}></div>
</div>

<footer
  class={css({
    width: 'full',
    backgroundColor: 'dark.gray.950',
    position: 'relative',
    overflow: 'hidden',
  })}
>
  <div
    class={css({
      maxWidth: '[1200px]',
      marginX: 'auto',
      paddingX: '24px',
      paddingTop: '64px',
      paddingBottom: '24px',
      position: 'relative',
    })}
  >
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: '2fr 1fr 1fr',
        gap: '48px',
        marginBottom: '64px',
      })}
    >
      <div>
        <div class={flex({ align: 'center', gap: '16px', marginBottom: '16px' })}>
          <Wordmark class={css({ height: '20px', color: 'dark.gray.50' })} />
          <div class={flex({ gap: '12px' })}>
            <a
              class={css({
                width: '[32px]',
                height: '[32px]',
                borderRadius: 'full',
                color: 'dark.gray.400',
                backgroundColor: 'dark.gray.900',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'all',
                _hover: {
                  backgroundColor: 'dark.gray.800',
                },
              })}
              aria-label="Twitter"
              href="https://twitter.com/typieofficial"
            >
              <Icon icon={TwitterIcon} size={16} />
            </a>
            <a
              class={css({
                width: '[32px]',
                height: '[32px]',
                borderRadius: 'full',
                color: 'dark.gray.400',
                backgroundColor: 'dark.gray.900',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'all',
                _hover: {
                  backgroundColor: 'dark.gray.800',
                },
              })}
              aria-label="GitHub"
              href="https://github.com/penxle"
            >
              <Icon icon={GithubIcon} size={16} />
            </a>
          </div>
        </div>

        <div
          class={css({
            fontSize: '13px',
            color: 'dark.gray.500',
            fontFamily: 'Pretendard',
            lineHeight: '[1.6]',
          })}
        >
          <p>(주) 펜슬컴퍼니</p>
          <p>대표: 배준현</p>
          <p>개인정보관리책임자: 배준현</p>
          <p>사업자등록번호: 610-88-03078</p>
          <p>통신판매업신고번호: 2023-서울강남-4541</p>
          <p>주소: 서울특별시 강남구 강남대로100길 14, 6층</p>
          <p>전화: 02-565-7695</p>
        </div>
      </div>

      <div>
        <h4
          class={css({
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'dark.gray.300',
            fontFamily: 'Pretendard',
            marginBottom: '16px',
            textTransform: 'uppercase',
            letterSpacing: '[0.05em]',
          })}
        >
          서비스
        </h4>
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <a
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              fontFamily: 'Pretendard',
              transition: 'colors',
              _hover: {
                color: 'dark.gray.200',
              },
            })}
            href="/"
          >
            소개
          </a>
          <a
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              fontFamily: 'Pretendard',
              transition: 'colors',
              _hover: {
                color: 'dark.gray.200',
              },
            })}
            href="/pricing"
          >
            요금제
          </a>
          <a
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              fontFamily: 'Pretendard',
              transition: 'colors',
              _hover: {
                color: 'dark.gray.200',
              },
            })}
            href="https://typie.link/help"
            rel="noopener noreferrer"
            target="_blank"
          >
            고객센터
          </a>
        </div>
      </div>

      <div>
        <h4
          class={css({
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'dark.gray.300',
            fontFamily: 'Pretendard',
            marginBottom: '16px',
            textTransform: 'uppercase',
            letterSpacing: '[0.05em]',
          })}
        >
          정책
        </h4>
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <a
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              fontFamily: 'Pretendard',
              transition: 'colors',
              _hover: {
                color: 'dark.gray.200',
              },
            })}
            href="https://help.typie.co/legal/terms"
            rel="noopener noreferrer"
            target="_blank"
          >
            이용약관
          </a>
          <a
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              fontFamily: 'Pretendard',
              transition: 'colors',
              _hover: {
                color: 'dark.gray.200',
              },
            })}
            href="https://help.typie.co/legal/privacy"
            rel="noopener noreferrer"
            target="_blank"
          >
            개인정보처리방침
          </a>
        </div>
      </div>
    </div>
  </div>
</footer>
