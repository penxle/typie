<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import FileIcon from '~icons/lucide/file';

  type Props = { title: string; lines: readonly string[]; entity: string; time: string };

  let { title, lines, entity, time }: Props = $props();

  const content = $derived([title, ...lines.map((line) => `- ${line}`)].join('\n'));

  const cardClass = flex({
    gap: '10px',
    width: '[fit-content]',
    maxWidth: 'full',
    padding: '12px',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '10px',
    backgroundColor: 'surface.default',
    boxShadow: 'sm',
  });
  const dotClass = css({
    flexShrink: '0',
    marginTop: '3px',
    size: '16px',
    borderWidth: '2px',
    borderColor: 'palette.gray',
    borderRadius: 'full',
  });
  const bodyClass = flex({ direction: 'column', gap: '4px', minWidth: '0' });
  const contentClass = css({
    fontFamily: 'landing',
    fontSize: '14px',
    lineHeight: '[1.55]',
    color: 'text.default',
    whiteSpace: 'pre-wrap',
    wordBreak: 'keep-all',
  });
  const metaClass = flex({ align: 'center', gap: '6px', fontSize: '12px', color: 'text.hint' });
  const entityClass = flex({ align: 'center', gap: '4px', fontWeight: 'medium', minWidth: '0' });
</script>

<div class={cardClass}>
  <span class={dotClass}></span>

  <div class={bodyClass}>
    <p class={contentClass}>{content}</p>

    <div class={metaClass}>
      <span class={entityClass}>
        <Icon icon={FileIcon} size={12} />
        <span class={css({ lineClamp: '1' })}>{entity}</span>
      </span>
      <span>·</span>
      <span class={css({ flexShrink: '0' })}>{time}</span>
    </div>
  </div>
</div>
