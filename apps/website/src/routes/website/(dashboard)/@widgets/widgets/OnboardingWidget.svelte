<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import ShapesIcon from '~icons/lucide/shapes';
  import Widget from '../Widget.svelte';

  type Props = {
    widgetId: string;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { widgetId }: Props = $props();
</script>

<Widget editMode={true} icon={ShapesIcon} title="새로운 위젯 기능">
  <div class={flex({ flexDirection: 'column', alignItems: 'center' })}>
    <div
      class={css({
        width: 'full',
        height: '130px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      })}
    >
      <svg style="width: 160px; height: 130px;" fill="none" viewBox="0 0 160 130" xmlns="http://www.w3.org/2000/svg">
        <!-- 위젯 1 (위) -->
        <rect fill="white" height="38" rx="5" stroke="#e5e5e5" stroke-width="1" width="100" x="10" y="5" />
        <path d="M 10 5 L 10 10 Q 10 5 15 5 L 105 5 Q 110 5 110 10 L 110 15 L 10 15 Z" fill="#f5f5f5" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="50" x="16" y="20" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="45" x="16" y="25" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="55" x="16" y="30" />

        <!-- 위젯 3 (아래, 큰 것) -->
        <rect fill="white" height="46" rx="5" stroke="#e5e5e5" stroke-width="1" width="100" x="10" y="71" />
        <path d="M 10 71 L 10 76 Q 10 71 15 71 L 105 71 Q 110 71 110 76 L 110 81 L 10 81 Z" fill="#f5f5f5" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="60" x="16" y="86" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="70" x="16" y="91" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="55" x="16" y="96" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="50" x="16" y="101" />

        <!-- 위젯 2 (중간, 오른쪽으로 이동, 맨 위) -->
        <rect fill="white" height="38" rx="5" stroke="#e5e5e5" stroke-width="1" width="100" x="55" y="38" />
        <path d="M 55 38 L 55 43 Q 55 38 60 38 L 150 38 Q 155 38 155 43 L 155 48 L 55 48 Z" fill="#f5f5f5" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="40" x="61" y="53" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="50" x="61" y="58" />
        <rect fill="#999" height="2" opacity="0.5" rx="1" width="35" x="61" y="63" />

        <!-- 커서 -->
        <g transform="translate(100, 42)">
          <path d="M 0 0 L 0 12 L 4 9 L 6 13.5 L 8 12.5 L 6 8 L 11 8 Z" fill="#333" opacity="0.8" />
          <path d="M 0 0 L 0 12 L 4 9 L 6 13.5 L 8 12.5 L 6 8 L 11 8 Z" fill="white" opacity="0.3" stroke="white" stroke-width="0.3" />
        </g>
      </svg>
    </div>

    <p class={css({ fontSize: '14px', color: 'text.default', textAlign: 'center', wordBreak: 'keep-all' })}>
      위젯으로 글쓰기를 더욱 편리하게 만들어보세요.
    </p>
  </div>
</Widget>
