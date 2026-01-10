<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { z } from 'zod';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import { Button, Icon, Modal } from '../../../components';
  import { createForm } from '../../../form';
  import { comma } from '../../../utils';
  import { getEditorContext, NodeView, NodeViewContentEditable } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes, HTMLAttributes }: Props = $props();

  const editorContext = getEditorContext();

  const isEditable = $derived(editor?.current.isEditable ?? false);
  const nodeId = $derived(node.attrs.nodeId as string);
  const price = $derived(node.attrs.price as number);
  const hasPrice = $derived(price >= 100);

  const handlePurchase = () => {
    editorContext?.onPaywallPurchase?.(nodeId, price);
  };

  let modalOpen = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

  const MIN_PRICE = 100;
  const MAX_PRICE = 1_000_000;

  const form = createForm({
    schema: z.object({
      price: z
        .number({ error: (issue) => (issue.input === undefined ? '가격을 입력해주세요.' : '숫자를 입력해주세요.') })
        .min(MIN_PRICE, { error: `최소 ${comma(MIN_PRICE)} P 이상이어야 해요.` })
        .max(MAX_PRICE, { error: `최대 ${comma(MAX_PRICE)} P까지 설정할 수 있어요.` })
        .multipleOf(100, { error: '100 P 단위로 입력해주세요.' }),
    }),
    onSubmit: (data) => {
      updateAttributes({ price: data.price });
      modalOpen = false;
    },
  });

  const openModal = () => {
    form.reset();
    form.fields.price = hasPrice ? price : (undefined as unknown as number);
    modalOpen = true;
    setTimeout(() => inputEl?.focus(), 0);
  };

  const handlePriceChange = (e: Event) => {
    const target = e.target as HTMLInputElement;
    const value = target.value.replaceAll(/[^\d]/g, '');
    form.fields.price = value ? Number.parseInt(value, 10) : (undefined as unknown as number);
  };
</script>

<NodeView {...HTMLAttributes}>
  {#if isEditable}
    <div
      class={css({
        borderWidth: '2px',
        borderStyle: 'dashed',
        borderColor: 'border.strong',
        borderRadius: '8px',
      })}
    >
      <button
        class={flex({
          width: 'full',
          alignItems: 'center',
          gap: '8px',
          paddingX: '16px',
          paddingY: '10px',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.subtle' },
        })}
        contenteditable={false}
        onclick={openModal}
        type="button"
      >
        <Icon icon={LockKeyholeIcon} size={16} />
        <div class={flex({ alignItems: 'baseline', gap: '4px' })}>
          <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>유료 블록</span>
          {#if hasPrice}
            <span class={css({ fontSize: '14px', fontWeight: 'bold', color: 'text.muted' })}>({comma(price)} P)</span>
          {:else}
            <span class={css({ fontSize: '14px', fontWeight: 'bold', color: 'text.danger' })}>(가격 설정)</span>
          {/if}
        </div>
      </button>

      <NodeViewContentEditable style={css.raw({ paddingX: '16px', paddingY: '16px' })} />
    </div>

    <Modal style={css.raw({ padding: '24px', maxWidth: '440px' })} bind:open={modalOpen}>
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>가격 설정</h2>
          <p class={css({ fontSize: '14px', color: 'text.subtle', lineHeight: '[1.6]' })}>
            이 블록을 열람하기 위해 필요한 포인트를 설정해주세요.
          </p>
        </div>

        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={flex({ flexDirection: 'column', gap: '4px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <input
                bind:this={inputEl}
                class={css({
                  flexGrow: '1',
                  paddingX: '12px',
                  paddingY: '10px',
                  fontSize: '16px',
                  textAlign: 'right',
                  borderWidth: '1px',
                  borderColor: form.errors.price ? 'border.danger' : 'border.default',
                  borderRadius: '6px',
                  backgroundColor: 'surface.default',
                  _focus: { borderColor: form.errors.price ? 'border.danger' : 'border.brand', outline: 'none' },
                })}
                oninput={handlePriceChange}
                onkeydown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    form.handleSubmit();
                  }
                }}
                type="text"
                value={typeof form.fields.price === 'number' ? comma(form.fields.price) : ''}
              />
              <span class={css({ fontSize: '16px', color: 'text.muted' })}>P</span>
            </div>
            {#if form.errors.price}
              <p class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.price}</p>
            {/if}
          </div>

          <div
            class={css({
              padding: '12px',
              fontSize: '12px',
              color: 'text.muted',
              backgroundColor: 'surface.subtle',
              borderRadius: '6px',
            })}
          >
            <ul class={css({ display: 'flex', flexDirection: 'column', gap: '4px', paddingLeft: '16px', listStyleType: 'disc' })}>
              <li>1 P는 1원과 같아요.</li>
              <li>가격은 100 P 단위로 설정할 수 있어요.</li>
              <li>최소 {comma(MIN_PRICE)} P부터 최대 {comma(MAX_PRICE)} P까지 설정 가능해요.</li>
            </ul>
          </div>
        </div>

        <div class={flex({ gap: '8px', justifyContent: 'flex-end' })}>
          <Button onclick={() => (modalOpen = false)} size="md" variant="secondary">취소</Button>
          <Button loading={form.state.isLoading} onclick={() => form.handleSubmit()} size="md" variant="primary">저장</Button>
        </div>
      </div>
    </Modal>
  {:else}
    <div
      class={css({
        borderWidth: '2px',
        borderStyle: 'dashed',
        borderColor: 'border.strong',
        borderRadius: '8px',
      })}
    >
      {#if price === -1}
        <div
          class={flex({
            width: 'full',
            alignItems: 'center',
            gap: '8px',
            paddingX: '16px',
            paddingY: '10px',
          })}
          contenteditable={false}
        >
          <Icon icon={LockKeyholeIcon} size={16} />
          <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>유료 블록</span>
        </div>
      {:else}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            gap: '16px',
            padding: '32px',
          })}
          contenteditable={false}
        >
          <Icon style={css.raw({ color: 'text.muted' })} icon={LockKeyholeIcon} size={32} />

          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>유료 블록</p>
            <p class={css({ fontSize: '13px', color: 'text.muted' })}>
              이 콘텐츠를 보려면 {comma(price)} P가 필요해요.
            </p>
          </div>

          <button
            class={css({
              paddingX: '16px',
              paddingY: '8px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'text.bright',
              backgroundColor: 'accent.brand.default',
              borderRadius: '6px',
              cursor: 'pointer',
              transition: 'common',
              _hover: { backgroundColor: 'accent.brand.hover' },
            })}
            onclick={handlePurchase}
            type="button"
          >
            결제하기
          </button>
        </div>
      {/if}

      <NodeViewContentEditable style={price === -1 ? css.raw({ paddingX: '16px', paddingY: '16px' }) : css.raw({ display: 'none' })} />
    </div>
  {/if}
</NodeView>
