import { Button, Column, Heading, Row, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail';

type Props = {
  code: string;
};

const Email = ({ code }: Props) => {
  return (
    <TypieEmail preview="타이피가 정식 출시되었어요!">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em] text-center">
        타이피가 정식 출시되었어요!
      </Heading>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5] text-center">
        타이피에 사전 등록해 주셔서 감사합니다.
        <br />
        기다려 주셨던 타이피가 정식 출시되었어요!
      </Text>

      <Row className="mb-[20px]">
        <Column align="center">
          <Button
            className="bg-zinc-950 text-white py-[15px] px-[30px] rounded-[4px] font-medium text-[15px] no-underline text-center box-border"
            href="https://typie.link/community"
          >
            타이피 바로가기
          </Button>
        </Column>
      </Row>

      <Text className="text-[16px] text-zinc-700 mb-[10px] leading-[1.5] text-center">
        결제 시 아래 할인 코드를 입력해 첫 달 무료 이용이 가능해요.
      </Text>

      <Text className="text-[14px] text-zinc-700 font-mono bg-zinc-100 p-[12px] rounded-[4px] break-all text-center">{code}</Text>

      <Text className="text-[16px] text-zinc-700 mt-[20px] leading-[1.5] text-center">
        언제나 더 편리하고 즐거운 글쓰기 경험을 선물해 드릴게요.
        <br />
        앞으로도 잘 부탁드려요. 감사합니다.
        <br />
        <br />- 타이피 팀 드림
      </Text>
    </TypieEmail>
  );
};

Email.PreviewProps = {
  code: '1234-5678-9012-3456-7890',
};

export default Email;
