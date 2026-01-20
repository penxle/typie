<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import { marked } from 'marked';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { inview } from '../(index)/inview';

  let { data } = $props();
</script>

<Helmet description="올인원 글쓰기 도구 타이피의 새로운 기능과 개선 사항들을 확인해보세요." title="업데이트 노트" />

<div
  class={css({
    position: 'relative',
    minHeight: '[100vh]',
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

  {#if !data.preview}
    <section
      class={css({
        position: 'relative',
        paddingTop: { sm: '100px', lg: '140px' },
        paddingBottom: { sm: '60px', lg: '80px' },
        paddingX: { sm: '24px', lg: '80px' },
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
      <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
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
          Changelog
        </span>

        <h1
          class={css({
            fontSize: { sm: '[36px]', lg: '[56px]' },
            fontWeight: 'medium',
            color: 'dark.gray.100',
            lineHeight: '[1.2]',
            letterSpacing: '[-0.02em]',
            fontFamily: 'Paperlogy',
            marginBottom: '20px',
          })}
        >
          새로운 기능,
          <br />
          <span class={css({ color: 'dark.gray.400' })}>더 나은 경험.</span>
        </h1>

        <p
          class={css({
            fontSize: { sm: '16px', lg: '18px' },
            color: 'dark.gray.400',
            lineHeight: '[1.65]',
            maxWidth: '[400px]',
          })}
        >
          타이피의 새로운 기능과 개선 사항들을 확인해보세요.
        </p>
      </div>
    </section>
  {/if}

  <section
    class={css({
      position: 'relative',
      paddingTop: data.preview ? '140px' : '0',
      paddingBottom: { sm: '80px', lg: '120px' },
      paddingX: { sm: '24px', lg: '80px' },
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div class={flex({ direction: 'column', gap: '0' })}>
        {#each data.entries as entry, idx (entry.id)}
          <article
            style:--delay={`${Math.min(idx * 0.06, 0.36)}s`}
            class={css({
              position: 'relative',
              display: 'grid',
              gridTemplateColumns: { sm: '1fr', lg: '[140px 1fr]' },
              gap: { sm: '0', lg: '48px' },
              paddingY: { sm: '40px', lg: '56px' },
              opacity: '0',
              transform: 'translate3d(0, 20px, 0)',
              transition:
                '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) var(--delay), transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) var(--delay)]',
              '&.in-view': {
                opacity: '100',
                transform: 'translate3d(0, 0, 0)',
              },
            })}
            {@attach inview}
          >
            <div
              class={css({
                position: { lg: 'sticky' },
                top: { lg: '140px' },
                alignSelf: { lg: 'start' },
                marginBottom: { sm: '20px', lg: '0' },
              })}
            >
              <div
                class={css({
                  display: 'flex',
                  flexDirection: { sm: 'row', lg: 'column' },
                  alignItems: { sm: 'center', lg: 'flex-start' },
                  gap: { sm: '12px', lg: '8px' },
                })}
              >
                <time
                  class={css({
                    fontSize: { sm: '13px', lg: '14px' },
                    fontFamily: 'mono',
                    color: 'dark.gray.400',
                    letterSpacing: '[0.02em]',
                  })}
                >
                  {dayjs(entry.date).formatAsDate()}
                </time>
              </div>
            </div>

            <div class={css({ minWidth: '0' })}>
              <h2
                class={css({
                  fontSize: { sm: '[24px]', lg: '[32px]' },
                  fontWeight: 'medium',
                  marginBottom: '24px',
                  color: 'dark.gray.100',
                  lineHeight: '[1.2]',
                  letterSpacing: '[-0.02em]',
                  fontFamily: 'Paperlogy',
                })}
              >
                {entry.title}
              </h2>

              {#if entry.image?.url}
                <div
                  class={css({
                    position: 'relative',
                    maxWidth: '[720px]',
                    marginBottom: '32px',
                  })}
                >
                  <div
                    class={css({
                      position: 'absolute',
                      top: '[-12px]',
                      left: '[-12px]',
                      width: '[calc(100% + 24px)]',
                      height: '[calc(100% + 24px)]',
                      borderWidth: '1px',
                      borderColor: 'dark.gray.900',
                      pointerEvents: 'none',
                      display: { sm: 'none', lg: 'block' },
                    })}
                  ></div>
                  <div
                    class={css({
                      position: 'relative',
                      backgroundColor: 'dark.gray.900',
                      padding: '4px',
                      borderWidth: '1px',
                      borderColor: 'dark.gray.900',
                    })}
                  >
                    <img
                      class={css({
                        width: 'full',
                        height: 'auto',
                        display: 'block',
                        objectFit: 'cover',
                        maxHeight: '[420px]',
                      })}
                      alt={entry.title}
                      loading="lazy"
                      src={entry.image.url}
                    />
                  </div>
                </div>
              {/if}

              <div
                class={css({
                  maxWidth: '[680px]',
                  fontSize: { sm: '15px', lg: '16px' },
                  lineHeight: '[1.75]',
                  color: 'dark.gray.300',
                  '& p': {
                    marginBottom: '20px',
                  },
                  '& p:last-child': {
                    marginBottom: '0',
                  },
                  '& h1, & h2, & h3': {
                    fontWeight: 'medium',
                    color: 'dark.gray.100',
                    fontFamily: 'Paperlogy',
                  },
                  '& h1': {
                    fontSize: '24px',
                    marginTop: '32px',
                    marginBottom: '16px',
                  },
                  '& h2': {
                    fontSize: '20px',
                    marginTop: '28px',
                    marginBottom: '14px',
                  },
                  '& h3': {
                    fontSize: '17px',
                    marginTop: '24px',
                    marginBottom: '12px',
                  },
                  '& h1:first-child, & h2:first-child, & h3:first-child': {
                    marginTop: '0',
                  },
                  '& ul, & ol': {
                    marginLeft: '24px',
                    marginBottom: '20px',
                  },
                  '& ul': {
                    listStyle: 'disc',
                  },
                  '& ol': {
                    listStyle: 'decimal',
                  },
                  '& li': {
                    marginBottom: '8px',
                    paddingLeft: '4px',
                  },
                  '& a': {
                    color: 'dark.brand.300',
                    textDecoration: 'underline',
                    textDecorationColor: '[oklch(0.55 0.18 70 / 0.5)]',
                    textUnderlineOffset: '3px',
                    transition: '[all 0.2s ease]',
                    _hover: {
                      color: 'dark.brand.200',
                      textDecorationColor: 'dark.brand.300',
                    },
                  },
                  '& strong, & b': {
                    fontWeight: 'semibold',
                    color: 'dark.gray.100',
                  },
                  '& em, & i': {
                    fontStyle: 'italic',
                  },
                  '& del, & s': {
                    textDecoration: 'line-through',
                    opacity: '[0.6]',
                  },
                  '& code': {
                    backgroundColor: 'dark.gray.800',
                    color: 'dark.gray.200',
                    paddingX: '7px',
                    paddingY: '3px',
                    fontSize: '14px',
                    fontFamily: 'mono',
                    borderRadius: '[2px]',
                  },
                  '& pre': {
                    backgroundColor: 'dark.gray.900',
                    color: 'dark.gray.200',
                    padding: '24px',
                    marginY: '24px',
                    overflow: 'auto',
                    borderWidth: '1px',
                    borderColor: 'dark.gray.900',
                  },
                  '& pre code': {
                    backgroundColor: 'transparent',
                    padding: '0',
                    fontSize: '13px',
                    lineHeight: '[1.7]',
                    color: '[inherit]',
                    borderRadius: '0',
                  },
                  '& blockquote': {
                    borderLeftWidth: '2px',
                    borderLeftColor: 'dark.brand.400',
                    paddingLeft: '24px',
                    marginY: '24px',
                    color: 'dark.gray.400',
                    fontStyle: 'italic',
                  },
                  '& hr': {
                    border: 'none',
                    borderTopWidth: '1px',
                    borderTopColor: 'dark.gray.900',
                    marginY: '40px',
                  },
                  '& img': {
                    maxWidth: 'full',
                    height: 'auto',
                    marginY: '24px',
                    borderWidth: '1px',
                    borderColor: 'dark.gray.900',
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

      {#if data.totalPages > 1}
        <div
          class={css({
            marginTop: { sm: '48px', lg: '64px' },
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            gap: '8px',
          })}
        >
          {#if data.currentPage > 1}
            <a
              class={css({
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '40px',
                height: '40px',
                borderWidth: '1px',
                borderColor: 'dark.gray.900',
                color: 'dark.gray.400',
                transition: '[all 0.2s ease-out]',
                _hover: {
                  borderColor: 'dark.gray.700',
                  color: 'dark.gray.200',
                },
              })}
              aria-label="이전 페이지"
              href={`?page=${data.currentPage - 1}`}
            >
              <Icon icon={ChevronLeftIcon} size={18} />
            </a>
          {/if}

          <div
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '0',
            })}
          >
            {#each Array.from({ length: data.totalPages }, (_, i) => i) as pageIndex (pageIndex)}
              {#if pageIndex + 1 === 1 || pageIndex + 1 === data.totalPages || (pageIndex + 1 >= data.currentPage - 2 && pageIndex + 1 <= data.currentPage + 2)}
                <a
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    width: '40px',
                    height: '40px',
                    backgroundColor: pageIndex + 1 === data.currentPage ? 'dark.gray.800' : 'transparent',
                    color: pageIndex + 1 === data.currentPage ? 'dark.gray.100' : 'dark.gray.500',
                    fontSize: '14px',
                    fontFamily: 'mono',
                    transition: '[all 0.2s ease]',
                    _hover: {
                      color: 'dark.gray.200',
                    },
                  })}
                  href={`?page=${pageIndex + 1}`}
                >
                  {pageIndex + 1}
                </a>
              {:else if pageIndex + 1 === data.currentPage - 3 || pageIndex + 1 === data.currentPage + 3}
                <span
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    width: '40px',
                    height: '40px',
                    color: 'dark.gray.600',
                    fontSize: '14px',
                  })}
                >
                  ...
                </span>
              {/if}
            {/each}
          </div>

          {#if data.currentPage < data.totalPages}
            <a
              class={css({
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '40px',
                height: '40px',
                borderWidth: '1px',
                borderColor: 'dark.gray.900',
                color: 'dark.gray.400',
                transition: '[all 0.2s ease-out]',
                _hover: {
                  borderColor: 'dark.gray.700',
                  color: 'dark.gray.200',
                },
              })}
              aria-label="다음 페이지"
              href={`?page=${data.currentPage + 1}`}
            >
              <Icon icon={ChevronRightIcon} size={18} />
            </a>
          {/if}
        </div>
      {/if}
    </div>
  </section>
</div>
