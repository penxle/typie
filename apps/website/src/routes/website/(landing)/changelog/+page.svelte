<script lang="ts">
  import dayjs from 'dayjs';
  import { marked } from 'marked';
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  let { data } = $props();
  let articles = $state<HTMLElement[]>([]);

  onMount(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('in-view');
          }
        });
      },
      {
        threshold: 0.05,
        rootMargin: '0px 0px 100px 0px',
      },
    );

    articles.forEach((article) => {
      if (article) observer.observe(article);
    });

    return () => {
      articles.forEach((article) => {
        if (article) observer.unobserve(article);
      });
    };
  });
</script>

<div
  class={css({
    position: 'relative',
    paddingY: '120px',
    minHeight: '[100vh]',
    backgroundColor: 'white',
    backgroundImage: 'linear-gradient(to bottom, token(colors.white), token(colors.gray.50) 50%, token(colors.white))',
  })}
>
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      backgroundImage: 'radial-gradient(circle at 1px 1px, token(colors.gray.300) 1px, transparent 1px)',
      backgroundSize: '[40px 40px]',
      opacity: '[0.4]',
      pointerEvents: 'none',
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
    })}
  ></div>

  <div class={css({ position: 'relative', maxWidth: '[1024px]', marginX: 'auto', paddingX: { base: '24px', lg: '40px' } })}>
    <header class={css({ marginBottom: '100px', textAlign: 'center' })}>
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          backgroundColor: 'amber.50',
          color: 'amber.900',
          paddingX: '16px',
          paddingY: '6px',
          borderRadius: 'full',
          fontSize: '13px',
          fontWeight: 'semibold',
          marginBottom: '32px',
          border: '1px solid',
          borderColor: 'amber.200',
          letterSpacing: 'wide',
          textTransform: 'uppercase',
        })}
      >
        <span
          class={css({
            width: '6px',
            height: '6px',
            backgroundColor: 'amber.500',
            borderRadius: 'full',
            boxShadow: '[0 0 0 4px rgba(251, 191, 36, 0.2)]',
          })}
        ></span>
        업데이트 노트
      </div>

      <h1
        class={css({
          fontSize: { base: '[48px]', md: '[64px]' },
          fontWeight: 'extrabold',
          lineHeight: '[1.1]',
          marginBottom: '24px',
          color: 'gray.900',
          fontFamily: 'Paperlogy',
          letterSpacing: 'tight',
        })}
      >
        업데이트 노트
      </h1>
      <p
        class={css({
          fontSize: '18px',
          color: 'gray.600',
          lineHeight: '[1.7]',
          fontWeight: 'normal',
          fontFamily: 'Pretendard',
          maxWidth: '[600px]',
          marginX: 'auto',
        })}
      >
        타이피의 최신 업데이트와 개선사항을 확인하세요.
        <br />
        더 나은 글쓰기 경험을 위해 지속적으로 발전하고 있습니다.
      </p>
    </header>

    <div class={css({ position: 'relative', maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          position: 'absolute',
          left: '280px',
          top: '0',
          bottom: '0',
          width: '1px',
          backgroundColor: 'gray.200',
          zIndex: '0',
          '@media (max-width: 768px)': {
            left: '32px',
          },
        })}
      ></div>

      <div class={css({ position: 'relative', paddingBottom: '40px' })}>
        <div class={flex({ direction: 'column', gap: '0' })}>
          {#each data.entries as entry, index (entry.id)}
            <article
              bind:this={articles[index]}
              class={css({
                position: 'relative',
                display: 'grid',
                gridTemplateColumns: { base: '1fr', md: '240px 80px 1fr' },
                gap: '0',
                opacity: '0',
                transform: 'translateY(30px)',
                transition: '[opacity 0.8s ease-out, transform 0.8s ease-out]',
                marginBottom: '80px',
                '&:last-child': {
                  marginBottom: '0',
                },
                '&.in-view': {
                  opacity: '100',
                  transform: 'translateY(0)',
                },
              })}
            >
              <div
                class={css({
                  display: { base: 'none', md: 'block' },
                  paddingRight: '40px',
                  textAlign: 'right',
                  alignSelf: 'start',
                  position: 'sticky',
                  top: '120px',
                  zIndex: '1',
                })}
              >
                <time
                  class={css({
                    fontSize: '14px',
                    color: 'gray.600',
                    fontWeight: 'semibold',
                    lineHeight: '[1.5]',
                    display: 'block',
                    paddingTop: '32px',
                    paddingBottom: '32px',
                  })}
                >
                  {dayjs(entry.date).formatAsDate()}
                </time>
              </div>

              <div
                class={css({
                  display: 'flex',
                  justifyContent: 'center',
                  alignSelf: 'start',
                  position: 'sticky',
                  top: '120px',
                  zIndex: '1',
                })}
              >
                <div
                  class={css({
                    position: 'relative',
                    marginTop: '32px',
                    marginBottom: '32px',
                    width: '16px',
                    height: '16px',
                    borderRadius: 'full',
                    backgroundColor: 'white',
                    border: '3px solid',
                    borderColor: 'amber.400',
                    zIndex: '2',
                    boxShadow: '[0 0 0 6px rgba(251, 191, 36, 0.1)]',
                    transition: '[all 0.3s ease]',
                    '&:hover': {
                      transform: 'scale(1.3)',
                      boxShadow: '[0 0 0 10px rgba(251, 191, 36, 0.15)]',
                    },
                  })}
                ></div>

                <div
                  class={css({
                    position: 'absolute',
                    top: '40px',
                    left: '20px',
                    width: '24px',
                    height: '1px',
                    backgroundColor: 'gray.200',
                    display: { base: 'none', md: 'block' },
                  })}
                ></div>
              </div>

              <div
                class={css({
                  backgroundColor: 'white',
                  borderRadius: '[16px]',
                  border: '1px solid',
                  borderColor: 'gray.100',
                  padding: { base: '28px', md: '36px' },
                  boxShadow: '[0 1px 3px rgba(0, 0, 0, 0.04), 0 6px 24px rgba(0, 0, 0, 0.02)]',
                  transition: '[all 0.3s ease]',
                  position: 'relative',
                  overflow: 'hidden',
                  marginLeft: { base: '60px', md: '0' },
                  '&:hover': {
                    transform: 'translateY(-4px)',
                    boxShadow: '[0 4px 6px rgba(0, 0, 0, 0.06), 0 12px 48px rgba(0, 0, 0, 0.04)]',
                    borderColor: 'gray.200',
                  },
                  '&::before': {
                    content: '""',
                    position: 'absolute',
                    top: '0',
                    left: '0',
                    width: '4px',
                    height: 'full',
                    backgroundImage: 'linear-gradient(to bottom, token(colors.amber.400), token(colors.amber.300))',
                    opacity: '0',
                    transition: '[opacity 0.3s ease]',
                  },
                  '&:hover::before': {
                    opacity: '1',
                  },
                })}
              >
                <time
                  class={css({
                    fontSize: '13px',
                    color: 'gray.500',
                    marginBottom: '16px',
                    display: { base: 'inline-flex', md: 'none' },
                    alignItems: 'center',
                    gap: '8px',
                    fontWeight: 'medium',
                    letterSpacing: 'wide',
                    textTransform: 'uppercase',
                  })}
                >
                  {dayjs(entry.date).formatAsDate()}
                </time>

                <h2
                  class={css({
                    fontSize: { base: '24px', md: '28px' },
                    fontWeight: 'bold',
                    marginBottom: '24px',
                    color: 'gray.900',
                    lineHeight: '[1.4]',
                    fontFamily: 'Paperlogy',
                    letterSpacing: 'tight',
                  })}
                >
                  {entry.title}
                </h2>

                {#if entry.image?.url}
                  <div
                    class={css({
                      marginBottom: '28px',
                      borderRadius: '12px',
                      overflow: 'hidden',
                      border: '1px solid',
                      borderColor: 'gray.100',
                      backgroundColor: 'gray.50',
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
                    fontSize: '15px',
                    lineHeight: '[1.8]',
                    color: 'gray.700',
                    marginBottom: '20px',
                    '& p': {
                      marginBottom: '16px',
                    },
                    '& p:last-child': {
                      marginBottom: '0',
                    },
                    '& h1': {
                      fontSize: '20px',
                      fontWeight: 'bold',
                    },
                    '& h2': {
                      fontSize: '18px',
                      fontWeight: 'bold',
                    },
                    '& h3': {
                      fontSize: '16px',
                      fontWeight: 'semibold',
                    },
                    '& ul': {
                      marginLeft: '24px',
                      marginBottom: '16px',
                      listStyle: 'disc',
                    },
                    '& ol': {
                      marginLeft: '24px',
                      marginBottom: '16px',
                      listStyle: 'decimal',
                    },
                    '& li': {
                      lineHeight: '[1.6]',
                      marginBottom: '8px',
                    },
                    '& ul ul, & ol ul, & ul ol, & ol ol': {
                      marginTop: '8px',
                      marginBottom: '0',
                    },
                    '& a': {
                      color: 'amber.600',
                      fontWeight: 'medium',
                      transition: '[all 0.2s ease]',
                      '&:hover': {
                        color: 'amber.700',
                        textDecoration: 'underline',
                        textUnderlineOffset: '3px',
                      },
                    },
                    '& strong, & b': {
                      fontWeight: 'semibold',
                      color: 'gray.900',
                    },
                    '& em, & i': {
                      fontStyle: 'italic',
                    },
                    '& del, & s': {
                      textDecoration: 'line-through',
                      opacity: '[0.7]',
                    },
                    '& code': {
                      backgroundColor: 'gray.100',
                      paddingX: '6px',
                      paddingY: '2px',
                      borderRadius: '4px',
                      fontSize: '14px',
                      fontFamily: 'mono',
                      color: 'gray.800',
                    },
                    '& pre': {
                      backgroundColor: 'gray.900',
                      padding: '20px',
                      borderRadius: '8px',
                      overflow: 'auto',
                      marginBottom: '16px',
                      marginTop: '16px',
                    },
                    '& pre code': {
                      backgroundColor: 'transparent',
                      padding: '0',
                      fontSize: '14px',
                      lineHeight: '[1.5]',
                      color: 'gray.100',
                    },
                    '& blockquote': {
                      borderLeft: '3px solid',
                      borderColor: 'amber.400',
                      paddingLeft: '20px',
                      marginY: '20px',
                      fontStyle: 'italic',
                      color: 'gray.600',
                      backgroundColor: 'amber.50',
                      paddingY: '16px',
                      paddingRight: '20px',
                      borderRadius: '[0 8px 8px 0]',
                    },
                    '& hr': {
                      border: 'none',
                      borderTop: '1px solid',
                      borderColor: 'gray.200',
                      marginY: '32px',
                    },
                    '& img': {
                      maxWidth: 'full',
                      height: 'auto',
                      borderRadius: '8px',
                      marginY: '16px',
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
            left: '280px',
            bottom: '0',
            width: '1px',
            height: '30px',
            backgroundColor: 'gray.200',
            '@media (max-width: 768px)': {
              left: '32px',
            },
            '&::after': {
              content: '""',
              position: 'absolute',
              bottom: '0',
              left: '[-3.5px]',
              width: '8px',
              height: '8px',
              borderRadius: 'full',
              backgroundColor: 'gray.300',
            },
          })}
        ></div>
      </div>
    </div>
  </div>
</div>

<div class={css({ backgroundColor: 'gray.50' })}>
  <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'dark.gray.950' })}></div>
</div>
