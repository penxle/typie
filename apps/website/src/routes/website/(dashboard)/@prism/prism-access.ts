export type PrismAccessReason = 'ai_opt_in_required' | 'prism_beta_required' | 'subscription_required' | 'prism_credit_insufficient';

export const prismAccessUnavailableMessage = (reason: PrismAccessReason): string => {
  switch (reason) {
    case 'ai_opt_in_required': {
      return 'AI 기능이 비활성화되었습니다';
    }
    case 'prism_beta_required': {
      return '현재 베타 참여자만 사용할 수 있습니다';
    }
    case 'subscription_required': {
      return '구독이 필요합니다';
    }
    case 'prism_credit_insufficient': {
      return '크레딧이 부족합니다';
    }
  }
};
