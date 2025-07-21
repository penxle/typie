<script lang="ts">
  import dayjs from 'dayjs';
  import { marked } from 'marked';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  let { data } = $props();
</script>

<div class={css({ paddingY: '80px', minHeight: '[100vh]' })}>
  <div class={css({ maxWidth: '800px', marginX: 'auto', paddingX: '20px' })}>
    <header class={css({ marginBottom: '60px', textAlign: 'center' })}>
      <h1
        class={css({
          fontSize: '[48px]',
          fontWeight: 'bold',
          lineHeight: '[1.2]',
          marginBottom: '16px',
          color: 'text.default',
        })}
      >
        Changelog
      </h1>
      <p
        class={css({
          fontSize: '18px',
          color: 'text.subtle',
          lineHeight: '[1.6]',
        })}
      >
        타이피의 새로운 기능과 개선사항을 확인하세요
      </p>
    </header>

    <div class={css({ position: 'relative' })}>
      <div
        class={css({
          position: 'absolute',
          left: '30px',
          top: '0',
          bottom: '0',
          width: '2px',
          backgroundColor: 'border.subtle',
          zIndex: '0',
        })}
      ></div>

      <div class={flex({ direction: 'column', gap: '40px' })}>
        {#each data.entries as entry (entry.id)}
          <article class={css({ position: 'relative' })}>
            <div
              class={css({
                position: 'absolute',
                left: '24px',
                top: '20px',
                width: '14px',
                height: '14px',
                borderRadius: 'full',
                backgroundColor: 'surface.default',
                border: '3px solid',
                borderColor: 'accent.brand.default',
                zIndex: '1',
              })}
            ></div>

            <div
              class={css({
                marginLeft: '60px',
                backgroundColor: 'surface.default',
                borderRadius: '12px',
                border: '1px solid',
                borderColor: 'border.subtle',
                padding: '32px',
                transition: 'all',
                transitionDuration: '200ms',
                '&:hover': {
                  borderColor: 'border.default',
                  boxShadow: '[0 10px 15px -3px rgba(0, 0, 0, 0.1)]',
                },
              })}
            >
              <time class={css({ fontSize: '14px', color: 'text.subtle', marginBottom: '12px', display: 'block' })}>
                {dayjs(entry.date).formatAsDate()}
              </time>

              <h2
                class={css({
                  fontSize: '24px',
                  fontWeight: 'bold',
                  marginBottom: '16px',
                  color: 'text.default',
                  lineHeight: '[1.3]',
                })}
              >
                {entry.title}
              </h2>

              {#if entry.image?.url}
                <div
                  class={css({
                    marginBottom: '20px',
                    borderRadius: '8px',
                    overflow: 'hidden',
                    border: '1px solid',
                    borderColor: 'border.subtle',
                  })}
                >
                  <img
                    class={css({
                      width: 'full',
                      height: 'auto',
                      display: 'block',
                      objectFit: 'cover',
                      maxHeight: '300px',
                    })}
                    alt={entry.title}
                    src={entry.image.url}
                  />
                </div>
              {/if}

              <div
                class={css({
                  fontSize: '16px',
                  lineHeight: '[1.6]',
                  color: 'text.subtle',
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
                    '&:hover': {
                      color: 'text.default',
                      textDecoration: 'underline',
                      textUnderlineOffset: '2px',
                    },
                  },
                  '& strong, & b': {
                    fontWeight: 'semibold',
                    color: 'text.default',
                  },
                  '& em, & i': {
                    fontStyle: 'italic',
                  },
                  '& del, & s': {
                    textDecoration: 'line-through',
                    opacity: '[0.7]',
                  },
                  '& code': {
                    backgroundColor: 'surface.subtle',
                    paddingX: '6px',
                    paddingY: '2px',
                    borderRadius: '4px',
                    fontSize: '14px',
                    fontFamily: 'mono',
                  },
                  '& pre': {
                    backgroundColor: 'surface.subtle',
                    padding: '16px',
                    borderRadius: '8px',
                    overflow: 'auto',
                    marginBottom: '16px',
                  },
                  '& pre code': {
                    backgroundColor: 'transparent',
                    padding: '0',
                    fontSize: '14px',
                    lineHeight: '[1.5]',
                  },
                  '& blockquote': {
                    borderLeft: '4px solid',
                    borderColor: 'border.default',
                    paddingLeft: '20px',
                    marginY: '20px',
                    fontStyle: 'italic',
                    color: 'text.subtle',
                  },
                  '& hr': {
                    border: 'none',
                    borderTop: '1px solid',
                    borderColor: 'border.subtle',
                    marginY: '24px',
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
    </div>
  </div>
</div>
