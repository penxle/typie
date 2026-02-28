<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';

  function seededRandom(seed: number) {
    return () => {
      seed = (seed * 16_807 + 0) % 2_147_483_647;
      return (seed - 1) / 2_147_483_646;
    };
  }

  const rand = seededRandom(42);
  const variants = ['skeleton-typing-a', 'skeleton-typing-b', 'skeleton-typing-c'] as const;
  function generateLines(count: number) {
    return Array.from({ length: count }, () => {
      const duration = 3.5 + rand() * 1.5;
      return {
        width: `${60 + Math.floor(rand() * 36)}%`,
        animation: `pulse 2s ease-in-out infinite, ${variants[Math.floor(rand() * 3)]} ${duration}s ease-in-out ${-rand() * duration}s infinite`,
      };
    });
  }

  const linesBefore = generateLines(3);
  const linesAfter = generateLines(5);

  const bar = css({
    backgroundColor: 'surface.muted',
    borderRadius: '4px',
    animation: 'pulse 2s ease-in-out infinite',
    flexShrink: '0',
  });

  const textLine = css({
    backgroundColor: 'surface.muted',
    borderRadius: '4px',
    flexShrink: '0',
    transformOrigin: 'left',
  });
</script>

<!-- Header -->
<div class={css({ paddingTop: { base: '48px', md: '80px' } })}>
  <!-- Breadcrumb -->
  <div class={flex({ alignItems: 'center', gap: '6px', marginBottom: '20px' })}>
    <div style:width="18px" style:height="18px" style:border-radius="4px" class={bar}></div>
    <div style:width="60px" style:height="13px" class={bar}></div>
  </div>

  <!-- Title -->
  <div
    style:width="45%"
    style:height="1lh"
    class={css({
      fontSize: { base: '24px', lg: '28px' },
      backgroundColor: 'surface.muted',
      borderRadius: '4px',
      animation: 'pulse 2s ease-in-out infinite',
    })}
  ></div>

  <!-- Subtitle -->
  <div
    style:width="30%"
    style:height="1lh"
    style:margin-top="8px"
    class={css({
      fontSize: { base: '14px', lg: '16px' },
      backgroundColor: 'surface.muted',
      borderRadius: '4px',
      animation: 'pulse 2s ease-in-out infinite',
    })}
  ></div>

  <!-- Action row -->
  <div class={flex({ align: 'center', justify: 'space-between', marginTop: '24px', paddingBottom: '16px' })}>
    <div></div>
    <div class={flex({ align: 'center', gap: '12px' })}>
      <div style:width="20px" style:height="20px" class={bar}></div>
      <div style:width="20px" style:height="20px" class={bar}></div>
    </div>
  </div>

  <!-- Divider -->
  <div class={css({ height: '1px', backgroundColor: 'border.subtle', marginBottom: '24px' })}></div>
</div>

<!-- Body -->
<div style:font-size="16px" style:line-height="1.6" class={flex({ flexDirection: 'column', gap: '16px' })}>
  {#each linesBefore as line, i (i)}
    <div style:height="1lh" class={flex({ alignItems: 'center' })}>
      <div style:width={line.width} style:height="16px" style:animation={line.animation} class={textLine}></div>
    </div>
  {/each}

  <div
    class={css({
      backgroundColor: 'surface.muted',
      borderRadius: '8px',
      width: 'full',
      height: '320px',
      animation: 'pulse 2s ease-in-out infinite',
    })}
  ></div>

  {#each linesAfter as line, i (i)}
    <div style:height="1lh" class={flex({ alignItems: 'center' })}>
      <div style:width={line.width} style:height="16px" style:animation={line.animation} class={textLine}></div>
    </div>
  {/each}
</div>

<!-- Footer -->
<div
  class={flex({
    align: 'center',
    justify: 'space-between',
    marginTop: '20px',
    paddingBottom: '10px',
    width: 'full',
  })}
>
  <div style:width="20px" style:height="20px" class={bar}></div>
  <div class={flex({ align: 'center', gap: '12px' })}>
    <div style:width="20px" style:height="20px" class={bar}></div>
  </div>
</div>

<!-- ContentNavigation -->
<div class={css({ paddingBottom: { base: '60px', lg: '80px' } })}>
  <div
    class={flex({
      gap: '16px',
      marginTop: '40px',
      paddingTop: '24px',
      borderTopWidth: '1px',
      borderColor: 'border.subtle',
      width: 'full',
    })}
  >
    <div
      class={css({
        flex: '1',
        padding: '16px',
        borderRadius: '8px',
        backgroundColor: 'surface.subtle',
      })}
    >
      <div class={flex({ flexDirection: 'column', gap: '4px' })}>
        <div style:width="40px" style:height="12px" class={bar}></div>
        <div style:width="70%" style:height="14px" class={bar}></div>
      </div>
    </div>

    <div
      class={css({
        flex: '1',
        padding: '16px',
        borderRadius: '8px',
        backgroundColor: 'surface.subtle',
      })}
    >
      <div class={flex({ flexDirection: 'column', alignItems: 'flex-end', gap: '4px' })}>
        <div style:width="40px" style:height="12px" class={bar}></div>
        <div style:width="70%" style:height="14px" class={bar}></div>
      </div>
    </div>
  </div>
</div>
