import { Body, Container, Head, Html, Preview, Text } from '@react-email/components';

const Email = () => {
  return (
    <Html lang="ko">
      <Head />
      <Preview>테스트 이메일</Preview>
      <Body>
        <Container>
          <Text>테스트 이메일입니다</Text>
        </Container>
      </Body>
    </Html>
  );
};

export default Email;
