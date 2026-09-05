<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import GithubIcon from '~icons/simple-icons/github';
  import XIcon from '~icons/simple-icons/x';
  import WordmarkWhite from '$assets/logos/wordmark-white.svg?component';
</script>

<footer
  class={css({
    width: 'full',
    backgroundColor: 'surface.canvas',
    position: 'relative',
    borderTopWidth: '1px',
    borderTopColor: 'border.hairline',
    paddingX: { sm: '24px', lg: '80px' },
  })}
>
  <div
    class={css({
      position: 'absolute',
      left: { sm: '16px', lg: '48px' },
      top: '0',
      bottom: '0',
      width: '1px',
      backgroundColor: 'border.hairline',
      display: { sm: 'none', lg: 'block' },
    })}
  ></div>

  <div
    class={css({
      maxWidth: '[1200px]',
      marginX: 'auto',
      paddingTop: { sm: '60px', lg: '80px' },
      paddingBottom: { sm: '40px', lg: '60px' },
      position: 'relative',
    })}
  >
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '[2fr 1fr 1fr]' },
        gap: { sm: '48px', lg: '80px' },
        marginBottom: { sm: '48px', lg: '80px' },
      })}
    >
      <div>
        <div class={flex({ align: 'center', gap: '24px', marginBottom: '24px', flexWrap: 'wrap' })}>
          <WordmarkWhite class={css({ height: { sm: '18px', lg: '20px' } })} />
          <div class={flex({ gap: '8px' })}>
            <a
              class={css({
                width: '32px',
                height: '32px',
                borderWidth: '1px',
                borderColor: 'border.hairline',
                color: 'text.muted',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: '[all 0.2s ease]',
                _hover: {
                  borderColor: 'border.emphasis',
                  color: 'text.default',
                },
              })}
              aria-label="X"
              href="https://x.com/typieofficial"
            >
              <Icon icon={XIcon} size={14} />
            </a>
            <a
              class={css({
                width: '32px',
                height: '32px',
                borderWidth: '1px',
                borderColor: 'border.hairline',
                color: 'text.muted',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: '[all 0.2s ease]',
                _hover: {
                  borderColor: 'border.emphasis',
                  color: 'text.default',
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
            fontSize: '12px',
            lineHeight: '[1.7]',
            letterSpacing: '[0.02em]',
            color: 'text.hint',
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
            fontSize: '[11px]',
            fontFamily: 'mono',
            fontWeight: 'medium',
            color: 'text.muted',
            marginBottom: '20px',
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
          })}
        >
          Service
        </h4>
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/"
          >
            소개
          </a>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/pricing"
          >
            구독 안내
          </a>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/changelog"
          >
            업데이트 노트
          </a>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/download"
          >
            다운로드
          </a>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/open"
          >
            오픈 대시보드
          </a>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="https://penxle.channel.io"
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
            fontSize: '[11px]',
            fontFamily: 'mono',
            fontWeight: 'medium',
            color: 'text.muted',
            marginBottom: '20px',
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
          })}
        >
          Legal
        </h4>
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/legal/terms"
            rel="noopener noreferrer"
            target="_blank"
          >
            이용약관
          </a>
          <a
            class={css({
              fontSize: '14px',
              color: 'text.muted',
              transition: '[color 0.2s ease]',
              _hover: {
                color: 'text.default',
              },
            })}
            href="/legal/privacy"
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
