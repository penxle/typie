<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import DownloadIcon from '~icons/lucide/download';
  import { inview } from '../(index)/inview';

  const downloads = [
    { platform: 'macOS', variant: 'Apple Silicon', url: 'https://download.typie.net/desktop/Typie-mac-arm64.dmg' },
    { platform: 'macOS', variant: 'Intel', url: 'https://download.typie.net/desktop/Typie-mac-x64.dmg' },
    { platform: 'Windows', variant: '64비트', url: 'https://download.typie.net/desktop/Typie-win-x64.exe' },
  ];
</script>

<Helmet description="타이피 데스크톱 앱을 내려받아 글쓰기에 더 깊이 몰입하세요." title="타이피 데스크톱 앱" />

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
        Download
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
        타이피 데스크톱 앱
      </h1>

      <p
        class={css({
          fontSize: { sm: '16px', lg: '18px' },
          color: 'dark.gray.400',
          lineHeight: '[1.65]',
          maxWidth: '[480px]',
        })}
      >
        브라우저를 벗어나, 글쓰기만 남은 창에서.
      </p>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingBottom: { sm: '80px', lg: '120px' },
      paddingX: { sm: '24px', lg: '80px' },
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={cx(
          grid({ columns: { sm: 1, md: 3 }, gap: { sm: '16px', lg: '20px' } }),
          css({
            opacity: '0',
            transform: 'translate3d(0, 20px, 0)',
            transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
            '&.in-view': {
              opacity: '100',
              transform: 'translate3d(0, 0, 0)',
            },
          }),
        )}
        {@attach inview}
      >
        {#each downloads as download (download.url)}
          <a
            class={cx(
              'group',
              flex({
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: '16px',
                paddingX: { sm: '24px', lg: '28px' },
                paddingY: { sm: '24px', lg: '28px' },
                backgroundColor: 'dark.gray.900',
                borderWidth: '1px',
                borderColor: 'dark.gray.800',
                transition: '[all 0.2s ease-out]',
                _hover: {
                  borderColor: 'dark.gray.700',
                  backgroundColor: 'dark.gray.900/80',
                },
              }),
            )}
            href={download.url}
            rel="noopener"
          >
            <div class={flex({ flexDirection: 'column', gap: '4px' })}>
              <span class={css({ fontSize: '16px', fontWeight: 'medium', color: 'dark.gray.100' })}>{download.platform}</span>
              <span class={css({ fontSize: '13px', fontFamily: 'mono', color: 'dark.gray.500' })}>{download.variant}</span>
            </div>

            <Icon
              style={css.raw({
                color: 'dark.gray.500',
                transition: '[color 0.2s ease-out]',
                _groupHover: { color: 'dark.brand.300' },
              })}
              icon={DownloadIcon}
              size={20}
            />
          </a>
        {/each}
      </div>

      <p class={css({ marginTop: '24px', fontSize: '13px', color: 'dark.gray.500' })}>
        macOS 11 이상, Windows 10 이상에서 사용할 수 있어요.
      </p>
    </div>
  </section>
</div>
