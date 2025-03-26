import { Body, Button, Container, Head, Heading, Hr, Html, Img, Preview, Tailwind, Text } from '@react-email/components';

type Props = {
  resetUrl: string;
};

const Email = ({ resetUrl }: Props) => {
  return (
    <Html lang="ko">
      <Head>
        <style
          dangerouslySetInnerHTML={{
            __html: `
          @media (prefers-color-scheme: dark) {
            .logo-light { display: none !important; }
            .logo-dark { display: block !important; }
          }
          @media (prefers-color-scheme: light) {
            .logo-light { display: block !important; }
            .logo-dark { display: none !important; }
          }
        `,
          }}
        />
      </Head>
      <Preview>비밀번호를 재설정해 주세요</Preview>
      <Tailwind>
        <Body className="bg-white font-sans">
          <Container className="mx-auto py-[48px] px-[24px] max-w-[520px]">
            <Img
              src="https://cdn.glttr.io/email/logo-black.png"
              height="32"
              alt="글리터 로고"
              className="logo-light h-[32px] w-auto object-cover mb-[24px]"
            />
            <Img
              src="https://cdn.glttr.io/email/logo-white.png"
              height="32"
              alt="글리터 로고"
              className="logo-dark h-[32px] w-auto object-cover mb-[24px]"
              style={{ display: 'none' }}
            />

            <Heading className="text-[28px] font-bold text-[#111111] mb-[20px] tracking-[-0.02em]">비밀번호를 재설정해 주세요</Heading>

            <Text className="text-[16px] text-[#37352f] mb-[28px] leading-[1.5]">
              비밀번호 재설정을 요청하셨어요. 아래 버튼을 클릭해서 새로운 비밀번호를 설정해 주세요.
            </Text>

            <Button
              className="bg-[#000000] text-white py-[10px] px-[20px] rounded-[4px] font-medium text-[15px] no-underline text-center box-border"
              href={resetUrl}
            >
              비밀번호 재설정하기
            </Button>

            <Text className="text-[14px] text-[#6b6b6b] mt-[32px] mb-[10px] leading-[1.5]">
              버튼이 작동하지 않는다면, 아래 링크를 복사해서 브라우저에 붙여넣어 주세요:
            </Text>

            <Text className="text-[14px] text-[#37352f] font-mono bg-[#f1f1f1] p-[12px] rounded-[4px] break-all">{resetUrl}</Text>

            <Text className="text-[14px] text-[#6b6b6b] mt-[32px] leading-[1.5]">이 링크는 1시간 동안 유효해요.</Text>

            <Text className="text-[14px] text-[#6b6b6b] mt-[12px] leading-[1.5]">
              만약 비밀번호 재설정을 요청하지 않으셨다면, 이 이메일은 무시하셔도 돼요.
            </Text>

            <Hr className="border-[#eaeaea] my-[36px]" />

            <Text className="text-[12px] text-[#6b6b6b] m-0 text-left">(주)펜슬컴퍼니 | 서울특별시 강남구 강남대로100길 14, 6층</Text>
          </Container>
        </Body>
      </Tailwind>
    </Html>
  );
};

Email.PreviewProps = {
  resetUrl: 'https://glitter.im/auth/reset-password?code=123456',
};

export default Email;
