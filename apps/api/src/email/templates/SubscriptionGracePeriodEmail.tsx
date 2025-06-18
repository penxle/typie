import { Button, Heading, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail';

type Props = {
  userName: string;
  planName: string;
  gracePeriodEndsAt: string;
  dashboardUrl: string;
  reason: string;
};

const Email = ({ userName, planName, gracePeriodEndsAt, dashboardUrl, reason }: Props) => {
  return (
    <TypieEmail preview={`${gracePeriodEndsAt}까지 결제 정보를 업데이트해 주세요`}>
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">결제 정보 확인이 필요해요</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[28px] leading-[1.5]">
        {userName}님의 {planName} 구독 갱신을 위한 결제에 실패했어요. {gracePeriodEndsAt}까지 결제 정보를 업데이트해 주세요.
      </Text>

      <Text className="text-[14px] text-zinc-700 bg-zinc-100 p-[12px] rounded-[4px] mb-[28px]">사유: {reason}</Text>

      <Button
        className="bg-zinc-950 text-white py-[10px] px-[20px] rounded-[4px] font-medium text-[15px] no-underline text-center box-border"
        href={dashboardUrl}
      >
        결제 정보 업데이트
      </Button>
    </TypieEmail>
  );
};

Email.PreviewProps = {
  userName: '타이피',
  planName: '타이피 FULL ACCESS (월간)',
  gracePeriodEndsAt: '2025년 1월 1일',
  dashboardUrl: 'https://typie.co',
  reason: '한도 초과',
};

export default Email;
