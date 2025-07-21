<script lang="ts">
  import dayjs from 'dayjs';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  const formatDate = (dateString: string) => {
    return dayjs(dateString).formatAsDate();
  };
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
                {formatDate(entry.date)}
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

              <p
                class={css({
                  fontSize: '16px',
                  lineHeight: '[1.7]',
                  color: 'text.subtle',
                  marginBottom: '20px',
                })}
              >
                {entry.body}
              </p>
            </div>
          </article>
        {/each}
      </div>
    </div>
  </div>
</div>
