import { Button, Heading, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail';

type Props = {
  code: string;
};

const Email = ({ code }: Props) => {
  return (
    <TypieEmail preview="사전 예약 코드를 확인하세요">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">사전 예약 코드가 발송되었어요</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        타이피에 사전 등록해 주셔서 감사합니다.
        <br />
        기다려 주셨던 타이피가 출시되었습니다!
      </Text>

      <Button
        className="bg-zinc-950 text-white py-[10px] px-[20px] rounded-[4px] font-medium text-[15px] no-underline text-center box-border"
        href="https://typie.co/"
      >
        바로 이용하러 가기
      </Button>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        결제 시 아래 코드를 입력해 주시면 1개월 동안 무료로 사용하실 수 있어요.
      </Text>

      <Text className="text-[14px] text-zinc-700 font-mono bg-zinc-100 p-[12px] rounded-[4px] break-all">{code}</Text>
    </TypieEmail>
  );
};

Email.PreviewProps = {
  code: '1234-5678-9012-3456-7890',
};

export default Email;
