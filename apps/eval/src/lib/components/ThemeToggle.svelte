<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import IconMoon from '~icons/lucide/moon';
  import IconSun from '~icons/lucide/sun';
  import { browser } from '$app/environment';

  let theme = $state(browser ? (document.documentElement.dataset.theme ?? 'light') : 'light');

  const toggle = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    localStorage.setItem('theme', next);
    theme = next;
  };
</script>

<button
  class={css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    width: '32px',
    height: '32px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '6px',
    color: 'text.faint',
    cursor: 'pointer',
    transition: '[background-color 0.15s ease, color 0.15s ease]',
    _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
  })}
  aria-label={theme === 'dark' ? '라이트모드로 전환' : '다크모드로 전환'}
  onclick={toggle}
  title={theme === 'dark' ? '라이트모드로 전환' : '다크모드로 전환'}
  type="button"
>
  <Icon icon={theme === 'dark' ? IconSun : IconMoon} size={14} />
</button>
