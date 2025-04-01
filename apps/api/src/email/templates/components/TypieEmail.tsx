import { Body, Container, Head, Hr, Html, Img, Preview, Tailwind, Text } from '@react-email/components';

type Props = {
  preview: string;
  children: React.ReactNode;
};

const TypieEmail = ({ preview, children }: Props) => {
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
      <Preview>{preview}</Preview>
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

            {children}

            <Hr className="border-[#eaeaea] my-[36px]" />

            <Text className="text-[12px] text-[#6b6b6b] m-0 text-left">(주)펜슬컴퍼니 | 서울특별시 강남구 강남대로100길 14, 6층</Text>
          </Container>
        </Body>
      </Tailwind>
    </Html>
  );
};

export default TypieEmail;
