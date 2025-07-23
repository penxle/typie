<script lang="ts">
  import dayjs from 'dayjs';
  import { marked } from 'marked';
  import CalendarIcon from '~icons/lucide/calendar';
  import CircleFadingArrowUpIcon from '~icons/lucide/circle-fading-arrow-up';
  import { Helmet, Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  let { data } = $props();
</script>

<Helmet description="올인원 글쓰기 도구 타이피의 새로운 기능과 개선 사항들을 확인해보세요." title="업데이트 노트" />

<div
  class={css({
    position: 'relative',
    minHeight: '[100vh]',
    overflow: 'hidden',
    backgroundColor: 'white',
  })}
>
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      backgroundImage:
        'linear-gradient(to bottom, token(colors.white), token(colors.gray.50) 25%, token(colors.gray.50) 75%, token(colors.white))',
      zIndex: '0',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      inset: '0',
      backgroundImage: 'radial-gradient(circle at 1px 1px, token(colors.gray.200) 1px, transparent 1px)',
      backgroundSize: '[50px 50px]',
      opacity: '[0.3]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      top: '[15%]',
      right: '[10%]',
      width: '[600px]',
      height: '[600px]',
      backgroundImage: 'radial-gradient(circle, token(colors.amber.200), transparent)',
      opacity: '[0.6]',
      filter: '[blur(150px)]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      bottom: '[10%]',
      left: '[-5%]',
      width: '[500px]',
      height: '[500px]',
      backgroundImage: 'radial-gradient(circle, token(colors.gray.200), transparent)',
      opacity: '[0.7]',
      filter: '[blur(120px)]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <section
    class={css({
      position: 'relative',
      paddingY: { sm: '80px', lg: '100px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
      marginBottom: { sm: '80px', lg: '120px' },
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          backgroundColor: 'white',
          border: '4px solid',
          borderColor: 'gray.900',
          borderRadius: '0',
          paddingY: { sm: '40px', lg: '80px' },
          paddingX: { sm: '24px', lg: '60px' },
          position: 'relative',
          boxShadow: '[8px 8px 0 0 #000]',
          opacity: '0',
          transform: { sm: 'translateY(-40px) scale(0.95)', lg: 'translateY(-40px) rotate(-1deg) scale(0.95)' },
          transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'translateY(0) scale(1)', lg: 'translateY(0) rotate(0) scale(1)' },
          },
        })}
        data-observe
      >
        <div
          class={css({
            position: 'absolute',
            top: '0',
            left: '0',
            width: '40px',
            height: '40px',
            backgroundColor: 'amber.400',
            borderRight: '4px solid',
            borderBottom: '4px solid',
            borderColor: 'gray.900',
            transform: 'scale(0)',
            transformOrigin: 'top left',
            transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.8s]',
            '.in-view &': {
              transform: 'scale(1)',
            },
          })}
        ></div>
        <div
          class={css({
            position: 'absolute',
            bottom: '0',
            right: '0',
            width: '40px',
            height: '40px',
            backgroundColor: 'amber.400',
            borderTop: '4px solid',
            borderLeft: '4px solid',
            borderColor: 'gray.900',
            transform: 'scale(0)',
            transformOrigin: 'bottom right',
            transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.9s]',
            '.in-view &': {
              transform: 'scale(1)',
            },
          })}
        ></div>

        <div class={css({ textAlign: 'center' })}>
          <div
            class={css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: '8px',
              backgroundColor: 'gray.900',
              color: 'white',
              paddingY: '8px',
              paddingX: '24px',
              fontSize: '14px',
              fontWeight: 'bold',
              letterSpacing: '[0.1em]',
              marginBottom: '40px',
              transform: 'rotate(-2deg) scale(0)',
              transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.3s]',
              '.in-view &': {
                transform: 'rotate(-2deg) scale(1)',
              },
            })}
          >
            <Icon icon={CircleFadingArrowUpIcon} size={16} />
            FEATURES & IMPROVEMENTS
          </div>

          <h1
            class={css({
              fontSize: { sm: '[40px]', lg: '[80px]' },
              fontWeight: 'black',
              color: 'gray.900',
              fontFamily: 'Paperlogy',
              lineHeight: '[1]',
              textTransform: 'uppercase',
              marginBottom: '32px',
              opacity: '0',
              transform: 'translateY(20px)',
              transition:
                '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s]',
              '.in-view &': {
                opacity: '100',
                transform: 'translateY(0)',
              },
            })}
          >
            업데이트
            <br />
            <span
              class={css({
                backgroundColor: 'amber.400',
                paddingX: '20px',
                display: 'inline-block',
                transform: 'rotate(1deg) scale(0)',
                transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.6s]',
                '.in-view &': {
                  transform: 'rotate(1deg) scale(1)',
                },
              })}
            >
              노트
            </span>
          </h1>

          <p
            class={css({
              fontSize: { sm: '18px', lg: '20px' },
              fontWeight: 'medium',
              color: 'gray.700',
              maxWidth: '[600px]',
              marginX: 'auto',
              opacity: '0',
              transform: 'translateY(10px)',
              transition:
                '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s]',
              '.in-view &': {
                opacity: '100',
                transform: 'translateY(0)',
              },
            })}
          >
            새로운 기능과 개선 사항들을 한눈에 확인해봐요!
          </p>
        </div>
      </div>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingTop: '0',
      paddingBottom: { sm: '120px', lg: '160px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
    })}
  >
    <div class={css({ position: 'relative', maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          position: 'absolute',
          left: { sm: '20px', lg: '240px' },
          top: '0',
          bottom: '0',
          width: '4px',
          backgroundColor: 'gray.900',
          zIndex: '0',
          display: { sm: 'none', lg: 'block' },
        })}
      ></div>

      <div class={css({ position: 'relative', paddingBottom: '40px' })}>
        <div class={flex({ direction: 'column', gap: '60px' })}>
          {#each data.entries as entry (entry.id)}
            <article
              class={css({
                position: 'relative',
                display: 'grid',
                gridTemplateColumns: { sm: '1fr', lg: '220px 60px 1fr' },
                gap: '0',
                opacity: '0',
                transform: { sm: 'translateX(-10px)', lg: 'translateX(-10px) rotate(-0.5deg)' },
                transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                '&.in-view': {
                  opacity: '100',
                  transform: { sm: 'translateX(0)', lg: 'translateX(0) rotate(0)' },
                },
              })}
              data-observe
            >
              <div
                class={css({
                  display: { sm: 'none', lg: 'block' },
                  paddingRight: '32px',
                  textAlign: 'right',
                  alignSelf: 'start',
                  paddingTop: '16px',
                })}
              >
                <time
                  class={css({
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '6px',
                    backgroundColor: 'gray.900',
                    color: 'white',
                    paddingY: '6px',
                    paddingX: '16px',
                    fontSize: '14px',
                    fontWeight: 'bold',
                    transform: 'rotate(-2deg)',
                    letterSpacing: '[0.05em]',
                  })}
                >
                  <Icon icon={CalendarIcon} size={14} />
                  {dayjs(entry.date).formatAsDate()}
                </time>
              </div>

              <div
                class={css({
                  display: { sm: 'none', lg: 'flex' },
                  justifyContent: 'center',
                  alignSelf: 'start',
                  paddingTop: '20px',
                })}
              >
                <div
                  class={css({
                    position: 'relative',
                    width: '32px',
                    height: '32px',
                    backgroundColor: 'amber.400',
                    border: '4px solid',
                    borderColor: 'gray.900',
                    zIndex: '2',
                    transform: 'rotate(45deg) translateX(-11px) scale(0)',
                    transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.4s]',
                    '.in-view &': {
                      transform: 'rotate(45deg) translateX(-11px) scale(1)',
                    },
                    _hover: {
                      transform: 'rotate(45deg) translateX(-11px) scale(1.2)',
                    },
                  })}
                ></div>
              </div>

              <div
                class={css({
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  paddingY: { sm: '24px', lg: '40px' },
                  paddingX: { sm: '20px', lg: '32px' },
                  position: 'relative',
                  marginLeft: { sm: '0', lg: '32px' },
                  boxShadow: '[8px 8px 0 0 #000]',
                  transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  _hover: {
                    transform: { sm: 'translate(-2px, -2px)', lg: 'translate(-4px, -4px) rotate(0.5deg)' },
                    boxShadow: '[12px 12px 0 0 #000]',
                  },
                })}
              >
                <time
                  class={css({
                    display: { sm: 'inline-block', lg: 'none' },
                    backgroundColor: 'gray.900',
                    color: 'white',
                    paddingY: '4px',
                    paddingX: '12px',
                    fontSize: '12px',
                    fontWeight: 'bold',
                    letterSpacing: '[0.05em]',
                    marginBottom: '16px',
                    transform: 'rotate(-1deg)',
                  })}
                >
                  {dayjs(entry.date).formatAsDate()}
                </time>

                <div
                  class={css({
                    position: 'absolute',
                    top: '[-2px]',
                    right: '40px',
                    backgroundColor: 'amber.400',
                    color: 'gray.900',
                    paddingY: '8px',
                    paddingX: '16px',
                    fontSize: '12px',
                    fontWeight: 'bold',
                    textTransform: 'uppercase',
                    border: '4px solid',
                    borderColor: 'gray.900',
                    transform: 'translateY(-50%) rotate(2deg)',
                    letterSpacing: '[0.1em]',
                  })}
                >
                  UPDATE
                </div>

                <h2
                  class={css({
                    fontSize: { sm: '[24px]', lg: '[36px]' },
                    fontWeight: 'black',
                    marginBottom: '24px',
                    color: 'gray.900',
                    lineHeight: '[1.2]',
                    fontFamily: 'Paperlogy',
                    textTransform: 'uppercase',
                  })}
                >
                  {entry.title}
                </h2>

                {#if entry.image?.url}
                  <div
                    class={css({
                      marginBottom: '32px',
                      border: '4px solid',
                      borderColor: 'gray.900',
                      backgroundColor: 'gray.100',
                      padding: '4px',
                    })}
                  >
                    <img
                      class={css({
                        width: 'full',
                        height: 'auto',
                        display: 'block',
                        objectFit: 'cover',
                        maxHeight: '[400px]',
                      })}
                      alt={entry.title}
                      loading="lazy"
                      src={entry.image.url}
                    />
                  </div>
                {/if}

                <div
                  class={css({
                    fontSize: '16px',
                    lineHeight: '[1.8]',
                    color: 'gray.800',
                    fontWeight: 'medium',
                    '& p': {
                      marginBottom: '20px',
                    },
                    '& p:last-child': {
                      marginBottom: '0',
                    },
                    '& h1, & h2, & h3': {
                      fontWeight: 'black',
                    },
                    '& h1': {
                      fontSize: '24px',
                    },
                    '& h2': {
                      fontSize: '20px',
                    },
                    '& h3': {
                      fontSize: '18px',
                    },
                    '& ul, & ol': {
                      marginLeft: '24px',
                      marginBottom: '20px',
                    },
                    '& ul': {
                      listStyle: 'none',
                    },
                    '& ul li': {
                      position: 'relative',
                      paddingLeft: '20px',
                      '&::before': {
                        content: '"▪"',
                        position: 'absolute',
                        left: '0',
                        fontWeight: 'bold',
                      },
                    },
                    '& ol': {
                      listStyle: 'decimal',
                    },
                    '& li': {
                      marginBottom: '12px',
                      fontWeight: 'medium',
                    },
                    '& a': {
                      color: 'gray.900',
                      fontWeight: 'extrabold',
                      textDecoration: 'underline',
                      textDecorationThickness: '3px',
                      textDecorationColor: 'amber.400',
                      textUnderlineOffset: '2px',
                      transition: '[all 0.2s ease]',
                      _hover: {
                        backgroundColor: 'amber.400',
                        textDecoration: 'none',
                      },
                    },
                    '& strong, & b': {
                      fontWeight: 'black',
                      color: 'gray.900',
                    },
                    '& em, & i': {
                      fontStyle: 'normal',
                      backgroundColor: 'amber.400',
                      paddingX: '4px',
                    },
                    '& del, & s': {
                      textDecoration: 'line-through',
                      textDecorationThickness: '3px',
                      opacity: '[0.6]',
                    },
                    '& code': {
                      backgroundColor: 'gray.900',
                      color: 'white',
                      paddingX: '8px',
                      paddingY: '2px',
                      fontSize: '14px',
                      fontFamily: 'mono',
                      fontWeight: 'bold',
                    },
                    '& pre': {
                      backgroundColor: 'gray.900',
                      color: 'white',
                      padding: '24px',
                      border: '4px solid',
                      borderColor: 'gray.900',
                      marginY: '24px',
                      fontWeight: 'bold',
                      boxShadow: '[6px 6px 0 0 #fbbf24]',
                    },
                    '& pre code': {
                      backgroundColor: 'transparent',
                      padding: '0',
                      fontSize: '14px',
                      lineHeight: '[1.6]',
                    },
                    '& blockquote': {
                      borderLeft: '8px solid',
                      borderColor: 'amber.400',
                      backgroundColor: 'amber.50',
                      padding: '24px',
                      marginY: '24px',
                      fontWeight: 'semibold',
                      color: 'gray.900',
                      position: 'relative',
                      '&::before': {
                        content: '""',
                        position: 'absolute',
                        top: '8px',
                        left: '16px',
                        fontSize: '[48px]',
                        fontWeight: 'black',
                        color: 'amber.400',
                        opacity: '[0.5]',
                      },
                    },
                    '& hr': {
                      border: 'none',
                      borderTop: '4px solid',
                      borderColor: 'gray.900',
                      marginY: '40px',
                    },
                    '& img': {
                      maxWidth: 'full',
                      height: 'auto',
                      border: '4px solid',
                      borderColor: 'gray.900',
                      marginY: '24px',
                    },
                  })}
                >
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html marked(entry.body, { gfm: true, breaks: true })}
                </div>
              </div>
            </article>
          {/each}
        </div>

        <div
          class={css({
            position: 'absolute',
            left: { sm: '20px', lg: '240px' },
            bottom: '0',
            transform: 'translateX(-18px)',
            display: { sm: 'none', lg: 'block' },
          })}
        >
          <div
            class={css({
              width: '40px',
              height: '40px',
              backgroundColor: 'gray.900',
              transform: 'rotate(45deg)',
            })}
          ></div>
        </div>
      </div>
    </div>
  </section>
</div>
