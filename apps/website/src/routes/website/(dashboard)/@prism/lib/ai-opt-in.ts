import { Dialog } from '@typie/ui/notification';
import { SubscribeModal } from '../../@subscription/subscribe-modal.svelte';

export const AI_OPT_IN_FAILURE_MESSAGE = 'AI 설정을 바꾸지 못했어요. 잠시 후 다시 시도해 주세요';

export const promptAiOptIn = (enable: () => Promise<void>) => {
  Dialog.confirm({
    title: 'AI 기능을 활성화하시겠어요?',
    message:
      '사용자의 글은 AI 모델 학습에 절대 사용되지 않으며, 사용자가 요청할 때만 AI가 사용돼요. 언제든지 설정에서 비활성화할 수 있어요.',
    action: 'primary',
    actionLabel: '활성화',
    actionHandler: async () => {
      if (!SubscribeModal.gate('preferences_ai')) {
        return;
      }

      await enable();
    },
  });
};
