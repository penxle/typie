import { Body, Container, Head, Heading, Hr, Html, Img, Preview, Tailwind, Text } from '@react-email/components';

type Props = {
  name: string;
  newEmail: string;
};

const Email = ({ name, newEmail }: Props) => {
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
      <Preview>이메일 주소가 변경되었어요</Preview>
      <Tailwind>
        <Body className="bg-white font-sans">
          <Container className="mx-auto py-[48px] px-[24px] max-w-[520px]">
            <Img
              src="https://typie.net/email/logo-black.png"
              height="32"
              alt="타이피 로고"
              className="logo-light h-[32px] w-auto object-cover mb-[24px]"
            />
            <Img
              src="https://typie.net/email/logo-white.png"
              height="32"
              alt="타이피 로고"
              className="logo-dark h-[32px] w-auto object-cover mb-[24px]"
              style={{ display: 'none' }}
            />

            <Heading className="text-[28px] font-bold text-[#111111] mb-[20px] tracking-[-0.02em]">이메일 주소가 변경되었어요</Heading>

            <Text className="text-[16px] text-[#37352f] mb-[28px] leading-[1.5]">
              {name}님의 계정 이메일 주소가 {newEmail}로 변경되었어요.
              <br />
              혹시 변경한 적이 없다면 고객센터에 문의해 주세요.
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
  name: '타이피',
  newEmail: 'test@typie.co',
};

export default Email;
