import { Heading, Link, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail';

type Props = {
  userName: string;
  planName: string;
  expiredAt: string;
};

const Email = ({ userName, planName, expiredAt }: Props) => {
  return (
    <TypieEmail preview="타이피의 모든 기능을 다시 이용하려면 설정에서 구독을 재개해 주세요">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">구독이 중단되었어요</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        {userName}님의 {planName} 구독이 {expiredAt}자로 중단되었어요.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        구독이 중단되어 일부 기능이 제한되지만, 기존에 작성한 글은 계속 확인할 수 있어요.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[28px] leading-[1.5]">
        타이피의 모든 기능을 다시 이용하시려면 설정에서 구독을 재개해 주세요.
      </Text>

      <Text className="text-[14px] text-zinc-500 leading-[1.5]">
        궁금한 점이 있으시면{' '}
        <Link href="https://typie.link/help" className="text-zinc-500 underline">
          고객센터
        </Link>
        로 문의해 주세요.
      </Text>
    </TypieEmail>
  );
};

Email.PreviewProps = {
  userName: '타이피',
  planName: '타이피 FULL ACCESS (월간)',
  expiredAt: '2025년 1월 1일',
};

export default Email;
