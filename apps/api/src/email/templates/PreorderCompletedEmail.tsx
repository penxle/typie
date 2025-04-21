import { Button, Column, Heading, Row, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail';

const Email = () => {
  return (
    <TypieEmail preview="타이피에 사전 등록해 주셔서 감사합니다. 더 편리하고 즐거운 글쓰기 경험을 선물해 드릴게요.">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">사전 등록이 완료되었어요</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        타이피에 사전 등록해 주셔서 감사합니다.
        <br />
        더 편리하고 즐거운 글쓰기 경험을 선물해 드릴게요.
        <br />
        기다려 주셔서 감사드리며, 곧 만나요!
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        보내주신 모든 의견을 꼼꼼히 검토하고 알찬 서비스를 준비하고 있어요.
        <br />
        방향성은 바뀔 수 있으니, 언제든 다양한 의견 부탁드려요.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[40px] leading-[1.5]">
        사전 등록자를 대상으로 디스코드에서 타이피 커뮤니티가 운영되고 있어요.
        <br />
        개발 소식 업데이트부터 의견 수렴, 사용성 테스트까지 다양한 소통이 이루어지는 공간이에요.
        <br />
        관심 있으시다면 언제든 편히 둘러봐주세요.
      </Text>

      <Row className="mb-[40px]">
        <Column align="center">
          <Button
            className="bg-zinc-950 text-white py-[15px] px-[30px] rounded-[4px] font-medium text-[15px] no-underline text-center box-border"
            href="https://typie.link/community"
          >
            커뮤니티 가입하기
          </Button>
        </Column>
      </Row>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        앞으로도 중요한 소식은 이메일로 전달해드릴 예정이에요.
        <br />
        그럼, 다시 한번 잘 부탁드릴게요.
        <br />
        <br />- 타이피 팀 드림
      </Text>
    </TypieEmail>
  );
};

export default Email;
