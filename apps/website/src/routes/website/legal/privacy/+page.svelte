<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { marked } from 'marked';

  let { data } = $props();
</script>

<Helmet description="타이피 개인정보처리방침" title="개인정보처리방침" />

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

  <section
    class={css({
      position: 'relative',
      paddingY: { sm: '80px', lg: '120px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
    })}
  >
    <div class={css({ maxWidth: '[900px]', marginX: 'auto' })}>
      <div
        class={css({
          backgroundColor: 'white',
          border: '4px solid',
          borderColor: 'gray.900',
          borderRadius: '0',
          paddingY: { sm: '40px', lg: '60px' },
          paddingX: { sm: '24px', lg: '48px' },
          position: 'relative',
          boxShadow: '[8px 8px 0 0 #000]',
        })}
      >
        <h1
          class={css({
            fontSize: { sm: '[32px]', lg: '[48px]' },
            fontWeight: 'black',
            color: 'gray.900',
            fontFamily: 'Paperlogy',
            lineHeight: '[1]',
            textTransform: 'uppercase',
            marginBottom: '16px',
          })}
        >
          {data.document.title}
        </h1>

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
              color: 'gray.900',
              marginTop: '32px',
              marginBottom: '16px',
            },
            '& h1': {
              fontSize: '28px',
            },
            '& h2': {
              fontSize: '24px',
            },
            '& h3': {
              fontSize: '20px',
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
            },
            '& a': {
              color: 'gray.900',
              fontWeight: 'bold',
              textDecoration: 'underline',
              textDecorationThickness: '2px',
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
              fontStyle: 'italic',
            },
            '& blockquote': {
              borderLeft: '4px solid',
              borderColor: 'amber.400',
              backgroundColor: 'amber.50',
              padding: '20px',
              marginY: '24px',
              fontWeight: 'semibold',
              color: 'gray.900',
            },
            '& hr': {
              border: 'none',
              borderTop: '2px solid',
              borderColor: 'gray.300',
              marginY: '32px',
            },
          })}
        >
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html marked(data.document.body, { gfm: true, breaks: true })}
        </div>
      </div>
    </div>
  </section>
</div>
