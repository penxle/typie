<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import BellIcon from '~icons/lucide/bell';
  import GiftIcon from '~icons/lucide/gift';
  import MailIcon from '~icons/lucide/mail';
  import SparklesIcon from '~icons/lucide/sparkles';
  import ZapIcon from '~icons/lucide/zap';
  import { graphql } from '$mearie';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const [updateMarketingConsent] = createMutation(
    graphql(`
      mutation MarketingConsentModal_UpdateMarketingConsent_Mutation($input: UpdateMarketingConsentInput!) {
        updateMarketingConsent(input: $input) {
          id
          marketingConsent
          marketingConsentAskedAt
        }
      }
    `),
  );

  const handleConsent = async (consented: boolean) => {
    await updateMarketingConsent({ input: { marketingConsent: consented } });
    open = false;
    Toast.success(`${dayjs().formatAsDate()}에 마케팅 수신 ${consented ? '동의' : '거부'}처리됐어요.`);
  };
</script>

<Modal
  style={css.raw({
    alignItems: 'center',
    padding: '32px',
    maxWidth: '400px',
  })}
  closable={false}
  bind:open
>
  <div
    class={flex({
      alignItems: 'center',
      '& > div': {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        borderWidth: '2px',
        borderColor: 'surface.default',
        borderRadius: 'full',
        marginRight: '-8px',
        size: '32px',
        color: 'text.bright',
        backgroundColor: 'surface.dark',
      },
    })}
  >
    <div>
      <Icon icon={MailIcon} size={16} />
    </div>

    <div>
      <Icon icon={BellIcon} size={16} />
    </div>

    <div>
      <Icon icon={SparklesIcon} size={16} />
    </div>

    <div>
      <Icon icon={ZapIcon} size={16} />
    </div>

    <div>
      <Icon icon={GiftIcon} size={16} />
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', marginTop: '16px', textAlign: 'center' })}>
    <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>타이피 소식 받아보기</div>

    <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
      새 기능, 글쓰기 팁, 할인 혜택 등 다양한 소식을 전해드려요.
    </div>
  </div>

  <Button style={css.raw({ marginTop: '24px', width: 'full', height: '40px' })} gradient onclick={() => handleConsent(true)}>
    받을게요
  </Button>

  <Button style={css.raw({ marginTop: '8px', width: 'full', height: '40px' })} onclick={() => handleConsent(false)} variant="secondary">
    안 받을게요
  </Button>

  <div class={css({ marginTop: '16px', fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>나중에 설정에서 변경할 수 있어요</div>
</Modal>
