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

  const textLine = css({
    backgroundColor: 'surface.muted',
    borderRadius: '4px',
    flexShrink: '0',
    transformOrigin: 'left',
  });
</script>

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
