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
            .light-mode-logo { display: none !important; }
            .dark-mode-logo { display: block !important; }
          }
          @media (prefers-color-scheme: light) {
            .light-mode-logo { display: block !important; }
            .dark-mode-logo { display: none !important; }
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
              src="https://cdn.typie.net/email/logo-black.png?v=1"
              height="24"
              alt="타이피 로고"
              className="light-mode-logo h-[24px] w-auto object-cover mb-[24px]"
            />
            <Img
              src="https://cdn.typie.net/email/logo-white.png?v=1"
              height="24"
              alt="타이피 로고"
              className="dark-mode-logo h-[24px] w-auto object-cover mb-[24px]"
              style={{ display: 'none' }}
            />

            {children}

            <Hr className="border-zinc-200 my-[36px]" />

            <Text className="text-[12px] text-zinc-500 m-0 text-left">(주)펜슬컴퍼니 | 서울특별시 강남구 강남대로100길 14, 6층</Text>
          </Container>
        </Body>
      </Tailwind>
    </Html>
  );
};

export default TypieEmail;
