<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { marked } from 'marked';

  let { data } = $props();
</script>

<Helmet description="타이피 서비스 이용약관" title="이용약관" />

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

  <section
    class={css({
      position: 'relative',
      paddingTop: { sm: '100px', lg: '140px' },
      paddingBottom: { sm: '80px', lg: '120px' },
      paddingX: { sm: '24px', lg: '80px' },
    })}
  >
    <div class={css({ maxWidth: '[900px]', marginX: 'auto' })}>
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
        Legal
      </span>

      <h1
        class={css({
          fontSize: { sm: '[36px]', lg: '[56px]' },
          fontWeight: 'medium',
          color: 'dark.gray.100',
          lineHeight: '[1.2]',
          letterSpacing: '[-0.02em]',
          fontFamily: 'Paperlogy',
          marginBottom: '48px',
          paddingBottom: '48px',
          borderBottomWidth: '1px',
          borderBottomColor: 'dark.gray.900',
        })}
      >
        {data.document.title}
      </h1>

      <div
        class={css({
          fontSize: { sm: '15px', lg: '16px' },
          lineHeight: '[1.75]',
          color: 'dark.gray.300',
          wordBreak: 'keep-all',

          '& > *:first-child': {
            marginTop: '0',
          },
          '& > *:last-child': {
            marginBottom: '0',
          },

          '& p': {
            marginTop: '0',
            marginBottom: '24px',
          },

          '& h1': {
            fontSize: { sm: '28px', lg: '32px' },
            fontWeight: 'medium',
            fontFamily: 'Paperlogy',
            color: 'dark.gray.100',
            lineHeight: '[1.3]',
            letterSpacing: '[-0.01em]',
            marginTop: '64px',
            marginBottom: '24px',
            paddingBottom: '16px',
            borderBottomWidth: '1px',
            borderBottomColor: 'dark.gray.800',
          },
          '& h2': {
            fontSize: { sm: '22px', lg: '24px' },
            fontWeight: 'medium',
            fontFamily: 'Paperlogy',
            color: 'dark.gray.100',
            lineHeight: '[1.4]',
            letterSpacing: '[-0.01em]',
            marginTop: '56px',
            marginBottom: '20px',
          },
          '& h3': {
            fontSize: { sm: '18px', lg: '20px' },
            fontWeight: 'medium',
            fontFamily: 'Paperlogy',
            color: 'dark.gray.200',
            lineHeight: '[1.4]',
            marginTop: '40px',
            marginBottom: '16px',
          },
          '& h4': {
            fontSize: { sm: '16px', lg: '17px' },
            fontWeight: 'medium',
            color: 'dark.gray.200',
            lineHeight: '[1.5]',
            marginTop: '32px',
            marginBottom: '12px',
          },

          '& h1 + p, & h2 + p, & h3 + p, & h4 + p': {
            marginTop: '0',
          },

          '& ul, & ol': {
            marginTop: '0',
            marginBottom: '24px',
            paddingLeft: '24px',
          },
          '& ul': {
            listStyleType: 'disc',
          },
          '& ol': {
            listStyleType: 'decimal',
          },
          '& li': {
            marginBottom: '10px',
            paddingLeft: '4px',
          },
          '& li:last-child': {
            marginBottom: '0',
          },
          '& li::marker': {
            color: 'dark.gray.500',
          },
          '& li > ul, & li > ol': {
            marginTop: '10px',
            marginBottom: '0',
          },
          '& li > p': {
            marginBottom: '10px',
          },
          '& li > p:last-child': {
            marginBottom: '0',
          },

          '& a': {
            color: 'dark.brand.300',
            textDecoration: 'underline',
            textDecorationColor: 'dark.brand.300/40',
            textUnderlineOffset: '3px',
            transition: '[text-decoration-color 0.2s ease]',
            _hover: {
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

          '& blockquote': {
            borderLeftWidth: '3px',
            borderLeftColor: 'dark.gray.700',
            paddingLeft: '20px',
            paddingY: '4px',
            marginTop: '24px',
            marginBottom: '24px',
            color: 'dark.gray.400',
            fontStyle: 'italic',
          },
          '& blockquote p': {
            marginBottom: '12px',
          },
          '& blockquote p:last-child': {
            marginBottom: '0',
          },

          '& hr': {
            border: 'none',
            height: '1px',
            backgroundColor: 'dark.gray.800',
            marginTop: '48px',
            marginBottom: '48px',
          },

          '& table': {
            width: 'full',
            borderCollapse: 'collapse',
            marginTop: '24px',
            marginBottom: '24px',
            fontSize: '14px',
          },
          '& thead': {
            borderBottomWidth: '2px',
            borderBottomColor: 'dark.gray.700',
          },
          '& th': {
            paddingX: '16px',
            paddingY: '12px',
            fontWeight: 'medium',
            color: 'dark.gray.200',
            textAlign: 'left',
            backgroundColor: 'dark.gray.900/50',
          },
          '& td': {
            paddingX: '16px',
            paddingY: '12px',
            borderBottomWidth: '1px',
            borderBottomColor: 'dark.gray.800',
            verticalAlign: 'top',
          },
          '& tbody tr:last-child td': {
            borderBottomWidth: '0',
          },

          '& code': {
            fontFamily: 'mono',
            fontSize: '[0.9em]',
            backgroundColor: 'dark.gray.800',
            paddingX: '6px',
            paddingY: '2px',
            borderRadius: '4px',
            color: 'dark.gray.200',
          },
          '& pre': {
            backgroundColor: 'dark.gray.900',
            padding: '20px',
            borderRadius: '8px',
            overflowX: 'auto',
            marginTop: '24px',
            marginBottom: '24px',
          },
          '& pre code': {
            backgroundColor: 'transparent',
            padding: '0',
            borderRadius: '0',
          },
        })}
      >
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html marked(data.document.body, { gfm: true, breaks: true })}
      </div>
    </div>
  </section>
</div>
