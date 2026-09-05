<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import NumberFlow from '@number-flow/svelte';
  import { APP_STORE_URL, PLAY_STORE_URL } from '@typie/lib/const';
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import GlobeIcon from '~icons/lucide/globe';
  import AppleIcon from '~icons/simple-icons/apple';
  import AppStoreIcon from '~icons/simple-icons/appstore';
  import GooglePlayIcon from '~icons/simple-icons/googleplay';
  import WindowsIcon from '~icons/simple-icons/windows';
  import { browser } from '$app/environment';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$mearie';
  import heroImage from './images/hero.webp';
  import { inview } from './inview';

  const activeWritersQuery = createQuery(
    graphql(`
      query Hero_ActiveWriters_Query {
        activeWritersCount
      }
    `),
  );

  onMount(() => {
    const interval = setInterval(() => {
      activeWritersQuery.refetch();
    }, 30_000);

    return () => clearInterval(interval);
  });
</script>

<section
  class={css({
    position: 'relative',
    minHeight: { sm: '[calc(100dvh - 56px)]', lg: '[calc(100dvh - 64px)]' },
    backgroundColor: 'surface.canvas',
    display: 'flex',
    flexDirection: 'column',
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
      flex: '1',
      display: 'flex',
      flexDirection: 'column',
      justifyContent: 'center',
      paddingTop: { sm: '48px', lg: '48px' },
      paddingBottom: { sm: '48px', lg: '48px' },
      maxWidth: '[1200px]',
      marginX: 'auto',
      width: 'full',
    })}
  >
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '[1fr 1fr]' },
        gap: { sm: '48px', lg: '80px' },
        alignItems: 'center',
      })}
    >
      <div
        class={css({
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
        <div
          class={css({
            marginBottom: { sm: '32px', lg: '40px' },
          })}
        >
          <svg
            class={css({
              width: { sm: '[28px]', lg: '[40px]' },
              height: { sm: '[21px]', lg: '[30px]' },
              marginBottom: { sm: '16px', lg: '20px' },
            })}
            fill="none"
            viewBox="96 126 208 148"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              class={css({ fill: 'border.hairline' })}
              d="M249.82 166.581c4.034-13.581 12.781-26.582 21.392-35.33 1.929-1.957.571-5.251-2.187-5.251-31.524 0-59.001 24.651-73.466 61.141-8.965-11.016-22.289-18.576-37.474-20.236 3.993-13.703 12.808-26.838 21.487-35.654 1.929-1.957.571-5.251-2.187-5.251-37.283 0-68.889 34.438-80.0539 82.188.0407-.054-1.3311 7.87-1.3311 12.082 0 29.673 24.937 53.73 55.701 53.73 20.659 0 38.683-10.854 48.299-26.973C209.616 263.146 227.64 274 248.299 274 279.063 274 304 249.943 304 220.27s-24.136-52.92-54.207-53.689h.027Z"
            />
          </svg>
          <h1
            class={css({
              fontSize: { sm: '[42px]', lg: '[72px]' },
              fontWeight: 'semibold',
              color: 'text.default',
              lineHeight: '[1.2]',
              letterSpacing: '[-0.02em]',
              fontFamily: 'Paperlogy',
            })}
          >
            쓰는게 기다려지는
            <br />
            <span
              class={css({
                position: 'relative',
                display: 'inline',
              })}
            >
              <span
                class={css({
                  position: 'relative',
                  color: 'text.default',
                  _after: {
                    content: '""',
                    position: 'absolute',
                    left: '0',
                    right: '0',
                    bottom: '[2px]',
                    height: '[3px]',
                    backgroundColor: 'text.default',
                    opacity: '[0.6]',
                  },
                })}
              >
                글쓰기 도구.
              </span>
            </span>
          </h1>
        </div>

        <p
          class={css({
            fontSize: { sm: '16px', lg: '18px' },
            color: 'text.muted',
            lineHeight: '[1.65]',
            maxWidth: '[480px]',
            marginBottom: { sm: '36px', lg: '48px' },
          })}
        >
          글에만 집중할 수 있는 환경.
          <br />
          떠오를 때 바로, 어디서든 이어 쓰세요.
        </p>

        <div
          class={css({
            display: 'flex',
            flexDirection: { sm: 'column', lg: 'row' },
            alignItems: { sm: 'flex-start', lg: 'center' },
            gap: { sm: '20px', lg: '32px' },
          })}
        >
          <a
            class={cx(
              'group',
              css({
                display: 'inline-flex',
                alignItems: 'center',
                gap: '10px',
                paddingX: '28px',
                paddingY: '16px',
                fontSize: '15px',
                fontWeight: 'semibold',
                color: 'surface.default',
                backgroundColor: 'accent.default',
                transition: '[all 0.2s ease-out]',
                _hover: {
                  backgroundColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
                },
              }),
            )}
            href="/start"
          >
            시작하기
            <Icon
              style={css.raw({
                transition: '[transform 0.2s ease-out]',
                _groupHover: {
                  transform: 'translateX(4px)',
                },
              })}
              icon={ArrowRightIcon}
              size={16}
            />
          </a>

          <div
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '16px',
            })}
          >
            <span
              class={css({
                fontSize: '13px',
                color: 'text.muted',
                textTransform: 'uppercase',
                letterSpacing: '[0.08em]',
              })}
            >
              Available on
            </span>
            <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
              <a
                class={css({
                  color: 'text.muted',
                  transition: '[color 0.2s ease-out]',
                  _hover: { color: 'text.default' },
                })}
                href={env.PUBLIC_AUTH_URL}
              >
                <Icon icon={GlobeIcon} size={20} />
              </a>
              <a
                class={css({
                  color: 'text.muted',
                  transition: '[color 0.2s ease-out]',
                  _hover: { color: 'text.default' },
                })}
                href="/download"
              >
                <Icon icon={AppleIcon} size={20} />
              </a>
              <a
                class={css({
                  color: 'text.muted',
                  transition: '[color 0.2s ease-out]',
                  _hover: { color: 'text.default' },
                })}
                href="/download"
              >
                <Icon icon={WindowsIcon} size={20} />
              </a>
              <a
                class={css({
                  color: 'text.muted',
                  transition: '[color 0.2s ease-out]',
                  _hover: { color: 'text.default' },
                })}
                href={APP_STORE_URL}
                rel="noopener noreferrer"
                target="_blank"
              >
                <Icon icon={AppStoreIcon} size={20} />
              </a>
              <a
                class={css({
                  color: 'text.muted',
                  transition: '[color 0.2s ease-out]',
                  _hover: { color: 'text.default' },
                })}
                href={PLAY_STORE_URL}
                rel="noopener noreferrer"
                target="_blank"
              >
                <Icon icon={GooglePlayIcon} size={20} />
              </a>
            </div>
          </div>
        </div>
      </div>

      <div
        class={css({
          position: 'relative',
          opacity: '0',
          transform: 'translate3d(32px, 0, 0)',
          transition: '[opacity 0.9s cubic-bezier(0.16, 1, 0.3, 1) 0.2s, transform 0.9s cubic-bezier(0.16, 1, 0.3, 1) 0.2s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        {@attach inview}
      >
        <div
          class={css({
            position: 'absolute',
            top: '[-24px]',
            left: '[-24px]',
            width: '[calc(100% + 48px)]',
            height: '[calc(100% + 48px)]',
            borderWidth: '1px',
            borderColor: 'border.hairline',
            pointerEvents: 'none',
          })}
        ></div>

        <div
          class={css({
            position: 'relative',
            backgroundColor: 'surface.default',
            padding: '8px',
            borderWidth: '1px',
            borderColor: 'border.hairline',
          })}
        >
          <img
            class={css({
              width: 'full',
              height: 'auto',
              display: 'block',
            })}
            alt="타이피 에디터"
            src={heroImage}
          />
        </div>
      </div>
    </div>
  </div>

  <div
    class={css({
      marginX: { sm: '[-24px]', lg: '[-80px]' },
      paddingX: { sm: '24px', lg: '80px' },
      paddingY: { sm: '24px', lg: '32px' },
      borderTopWidth: '1px',
      borderTopColor: 'border.hairline',
      borderBottomWidth: '1px',
      borderBottomColor: 'border.hairline',
    })}
  >
    <div
      class={css({
        maxWidth: '[1200px]',
        marginX: 'auto',
        display: 'flex',
        flexDirection: { sm: 'column', lg: 'row' },
        justifyContent: 'space-between',
        alignItems: { sm: 'flex-start', lg: 'center' },
        gap: '16px',
      })}
    >
      <p
        class={css({
          fontSize: '13px',
          color: 'text.hint',
          fontFamily: 'mono',
        })}
      >
        글 쓰는 사람들이 만들었어요
      </p>

      <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
        <span
          class={css({
            width: '[8px]',
            height: '[8px]',
            borderRadius: 'full',
            backgroundColor: 'success.default',
          })}
        ></span>
        <span class={css({ fontSize: '13px', color: 'text.hint', fontFamily: 'mono' })}>
          {#if browser}
            <NumberFlow
              class={css({ color: 'text.muted', fontWeight: 'semibold' })}
              value={activeWritersQuery.data?.activeWritersCount ?? 0}
            />
          {:else}
            <strong class={css({ color: 'text.muted', fontWeight: 'semibold' })}>
              {(activeWritersQuery.data?.activeWritersCount ?? 0).toLocaleString()}
            </strong>
          {/if}
          명이 이번 주 글 쓰는 중
        </span>
      </div>
    </div>
  </div>
</section>
