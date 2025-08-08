<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import GithubIcon from '~icons/simple-icons/github';
  import TwitterIcon from '~icons/simple-icons/twitter';
  import WordmarkWhite from '$assets/logos/wordmark-white.svg?component';
</script>

<footer
  class={css({
    width: 'full',
    backgroundColor: 'gray.900',
    position: 'relative',
    overflow: 'hidden',
    borderTop: '4px solid',
    borderColor: 'gray.900',
  })}
>
  <div
    class={css({
      maxWidth: '[1200px]',
      marginX: 'auto',
      paddingX: { sm: '16px', lg: '24px' },
      paddingTop: { sm: '60px', lg: '80px' },
      paddingBottom: { sm: '40px', lg: '60px' },
      position: 'relative',
    })}
  >
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '2fr 1fr 1fr' },
        gap: { sm: '32px', lg: '64px' },
        marginBottom: { sm: '40px', lg: '80px' },
      })}
    >
      <div>
        <div class={flex({ align: 'center', gap: { sm: '16px', lg: '20px' }, marginBottom: '24px', flexWrap: 'wrap' })}>
          <WordmarkWhite class={css({ height: { sm: '18px', lg: '20px' } })} />
          <div class={flex({ gap: '12px' })}>
            <a
              class={css({
                width: '32px',
                height: '32px',
                backgroundColor: 'gray.800',
                color: 'gray.500',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: '[color 0.2s ease, background-color 0.2s ease]',
                _hover: {
                  backgroundColor: 'gray.700',
                  color: 'gray.300',
                },
              })}
              aria-label="Twitter"
              href="https://twitter.com/typieofficial"
            >
              <Icon icon={TwitterIcon} size={14} />
            </a>
            <a
              class={css({
                width: '32px',
                height: '32px',
                backgroundColor: 'gray.800',
                color: 'gray.500',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: '[color 0.2s ease, background-color 0.2s ease]',
                _hover: {
                  backgroundColor: 'gray.700',
                  color: 'gray.300',
                },
              })}
              aria-label="GitHub"
              href="https://github.com/penxle"
            >
              <Icon icon={GithubIcon} size={14} />
            </a>
          </div>
        </div>

        <div
          class={css({
            fontSize: { sm: '11px', lg: '12px' },
            lineHeight: '[1.6]',
            color: 'gray.600',
            '& p': {
              marginBottom: '4px',
            },
            '& p:last-child': {
              marginBottom: '0',
            },
          })}
        >
          <p>(주) 펜슬컴퍼니</p>
          <p>대표: 배준현 | 개인정보관리책임자: 배준현</p>
          <p>사업자등록번호: 610-88-03078</p>
          <p>통신판매업신고번호: 2023-서울강남-4541</p>
          <p>서울특별시 강남구 강남대로100길 14, 6층</p>
          <p>02-565-7695</p>
        </div>
      </div>

      <div>
        <h4
          class={css({
            fontSize: { sm: '13px', lg: '14px' },
            fontWeight: 'bold',
            color: 'gray.200',
            marginBottom: { sm: '16px', lg: '20px' },
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
            paddingBottom: { sm: '8px', lg: '12px' },
            borderBottom: '3px solid',
            borderColor: 'gray.700',
          })}
        >
          서비스
        </h4>
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <a
            class={css({
              fontSize: { sm: '14px', lg: '15px' },
              color: 'gray.500',
              fontWeight: 'medium',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'gray.200',
                textDecoration: 'underline',
                textDecorationThickness: '2px',
                textUnderlineOffset: '4px',
              },
            })}
            href="/"
          >
            소개
          </a>
          <a
            class={css({
              fontSize: { sm: '14px', lg: '15px' },
              color: 'gray.500',
              fontWeight: 'medium',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'gray.200',
                textDecoration: 'underline',
                textDecorationThickness: '2px',
                textUnderlineOffset: '4px',
              },
            })}
            href="/pricing"
          >
            구독 안내
          </a>
          <a
            class={css({
              fontSize: { sm: '14px', lg: '15px' },
              color: 'gray.500',
              fontWeight: 'medium',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'gray.200',
                textDecoration: 'underline',
                textDecorationThickness: '2px',
                textUnderlineOffset: '4px',
              },
            })}
            href="/changelog"
          >
            업데이트 노트
          </a>
          <a
            class={css({
              fontSize: { sm: '14px', lg: '15px' },
              color: 'gray.500',
              fontWeight: 'medium',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'gray.200',
                textDecoration: 'underline',
                textDecorationThickness: '2px',
                textUnderlineOffset: '4px',
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
            fontSize: { sm: '13px', lg: '14px' },
            fontWeight: 'bold',
            color: 'gray.200',
            marginBottom: { sm: '16px', lg: '20px' },
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
            paddingBottom: { sm: '8px', lg: '12px' },
            borderBottom: '3px solid',
            borderColor: 'gray.700',
          })}
        >
          정책
        </h4>
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <a
            class={css({
              fontSize: { sm: '14px', lg: '15px' },
              color: 'gray.500',
              fontWeight: 'medium',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'gray.200',
                textDecoration: 'underline',
                textDecorationThickness: '2px',
                textUnderlineOffset: '4px',
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
              fontSize: { sm: '14px', lg: '15px' },
              color: 'gray.500',
              fontWeight: 'medium',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'gray.200',
                textDecoration: 'underline',
                textDecorationThickness: '2px',
                textUnderlineOffset: '4px',
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
