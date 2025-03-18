import { Body, Container, Head, Hr, Html, Img, Preview, Text } from '@react-email/components';

const Email = () => {
  return (
    <Html lang="ko">
      <Head>
        <style>
          {`
            @media (prefers-color-scheme: dark) {
              .logo-black {
                display: none;
              }
            }

            @media (prefers-color-scheme: light) {
              .logo-white {
                display: none;
              }
            }
          `}
        </style>
      </Head>
      <Preview>글리터 사전 등록이 완료되었어요!</Preview>
      <Body style={{ wordBreak: 'keep-all' }}>
        <Container style={{ maxWidth: '640px', margin: '0 auto', padding: '40px' }}>
          <Img src="https://cdn.glttr.io/email/logo-black.png" alt="글리터" width={60} className="logo-black" />
          <Img src="https://cdn.glttr.io/email/logo-white.png" alt="글리터" width={60} className="logo-white" />

          <Text style={{ fontSize: '24px', fontWeight: 600, textAlign: 'center', lineHeight: '1.5' }}>
            글리터 사전 등록이 완료되었어요!
          </Text>

          <Text style={{ marginTop: '40px', fontSize: '14px', fontWeight: 500, textAlign: 'center', lineHeight: '1.5' }}>
            글리터에 사전 등록해 주셔서 감사합니다.
            <br />
            더 편리하고 즐거운 글쓰기 경험을 선물해 드릴게요.
            <br />
            기다려 주셔서 감사드리며, 곧 만나요!
          </Text>

          <Hr style={{ margin: '40px 0' }} />

          <Text style={{ fontSize: '10px', color: '#ACB2B9', textAlign: 'left', lineHeight: '1.5' }}>
            (주) 펜슬컴퍼니
            <br />
            대표 : 배준현
            <br />
            서울특별시 강남구 강남대로100길 14, 6층
          </Text>
        </Container>
      </Body>
    </Html>
  );
};

export default Email;
