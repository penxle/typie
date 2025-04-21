import { Heading, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail';

type Props = {
  name: string;
  email: string;
};

const Email = ({ name, email }: Props) => {
  return (
    <TypieEmail preview="이메일 주소가 변경되었어요">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">이메일 주소가 변경되었어요</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[28px] leading-[1.5]">
        {name}님의 계정 이메일 주소가 {email}로 변경되었어요.
        <br />
        혹시 변경한 적이 없다면 고객센터에 문의해 주세요.
      </Text>
    </TypieEmail>
  );
};

Email.PreviewProps = {
  name: '타이피',
  email: 'test@typie.co',
};

export default Email;
