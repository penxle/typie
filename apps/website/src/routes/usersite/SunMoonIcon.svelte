<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    dark: boolean;
    size?: number;
  };

  let { dark, size = 20 }: Props = $props();

  const uid = $props.id();
  const maskId = `${uid}-moon`;
</script>

<svg
  class={css({
    display: 'block',
    '& .sun, & .beams, & .moon circle': { transformOrigin: 'center' },
    '& .sun': { transition: '[transform 500ms cubic-bezier(0.5, 1.25, 0.75, 1.25)]' },
    '& .beams': { transition: '[transform 500ms cubic-bezier(0.5, 1.5, 0.75, 1.25), opacity 500ms cubic-bezier(0.25, 0, 0.3, 1)]' },
    '& .moon circle': { transition: '[transform 250ms cubic-bezier(0, 0, 0, 1)]' },
    '&[data-dark] .sun': {
      transform: '[scale(1.75)]',
      transitionTimingFunction: '[cubic-bezier(0.25, 0, 0.3, 1)]',
      transitionDuration: '[250ms]',
    },
    '&[data-dark] .beams': { opacity: '0', transform: '[rotate(-25deg)]', transitionDuration: '[150ms]' },
    '&[data-dark] .moon circle': { transform: '[translateX(-7px)]', transitionDelay: '[250ms]', transitionDuration: '[500ms]' },
    _motionReduce: { '& .sun, & .beams, & .moon circle': { transition: '[none]' } },
  })}
  aria-hidden="true"
  data-dark={dark || undefined}
  height={size}
  viewBox="0 0 24 24"
  width={size}
>
  <mask id={maskId} class="moon">
    <rect fill="white" height="100%" width="100%" />
    <circle cx="24" cy="10" fill="black" r="6" />
  </mask>
  <circle class="sun" cx="12" cy="12" fill="currentColor" mask={`url(#${maskId})`} r="6" />
  <g class="beams" stroke="currentColor" stroke-linecap="round" stroke-width="2">
    <line x1="12" x2="12" y1="1" y2="3" />
    <line x1="12" x2="12" y1="21" y2="23" />
    <line x1="4.22" x2="5.64" y1="4.22" y2="5.64" />
    <line x1="18.36" x2="19.78" y1="18.36" y2="19.78" />
    <line x1="1" x2="3" y1="12" y2="12" />
    <line x1="21" x2="23" y1="12" y2="12" />
    <line x1="4.22" x2="5.64" y1="19.78" y2="18.36" />
    <line x1="18.36" x2="19.78" y1="5.64" y2="4.22" />
  </g>
</svg>
