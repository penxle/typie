<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { FAQS } from './pricing';

  let expanded = $state<number | null>(null);

  const toggle = (index: number) => {
    expanded = expanded === index ? null : index;
  };

  const itemClass = cx('group', css({ borderBottomWidth: '1px', borderBottomColor: 'border.hairline' }));
  const buttonClass = css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: '16px',
    width: 'full',
    paddingY: { base: '18px', lg: '22px' },
    textAlign: 'left',
    fontFamily: 'landing',
    fontSize: { base: '15px', lg: '16px' },
    fontWeight: 'medium',
    lineHeight: '[1.5]',
    color: 'text.muted',
    transition: '[color 0.2s ease-out]',
    _hover: { color: 'text.default' },
    _groupExpanded: { color: 'text.default' },
  });
  const chevronStyle = css.raw({
    flexShrink: '0',
    color: 'text.hint',
    transition: '[transform 0.2s ease-out]',
    _motionReduce: { transition: '[none]' },
    _groupExpanded: { transform: 'rotate(180deg)' },
  });
  const panelClass = css({
    display: 'grid',
    gridTemplateRows: '0fr',
    transition: '[grid-template-rows 0.2s ease-out]',
    _motionReduce: { transition: '[none]' },
    _groupExpanded: { gridTemplateRows: '1fr' },
  });
  const answerClass = css({
    paddingBottom: { base: '18px', lg: '22px' },
    fontFamily: 'landing',
    fontSize: '14px',
    lineHeight: '[1.7]',
    color: 'text.muted',
  });
</script>

<div>
  {#each FAQS as faq, index (faq.question)}
    <div class={itemClass} aria-expanded={expanded === index}>
      <button class={buttonClass} aria-expanded={expanded === index} onclick={() => toggle(index)} type="button">
        {faq.question}
        <Icon style={chevronStyle} icon={ChevronDownIcon} size={16} />
      </button>
      <div class={panelClass}>
        <div class={css({ overflow: 'hidden' })}>
          <p class={answerClass}>{faq.answer}</p>
        </div>
      </div>
    </div>
  {/each}
</div>
