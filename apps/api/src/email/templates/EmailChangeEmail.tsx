import TypieLinkEmail from './components/TypieLinkEmail';

type Props = {
  name: string;
  newEmail: string;
  verificationUrl: string;
};

const Email = ({ name, newEmail, verificationUrl }: Props) => {
  return (
    <TypieLinkEmail
      preview="이메일 주소를 인증해 주세요"
      heading="이메일 인증이 필요해요"
      text={`${name}님의 계정 이메일 주소를 ${newEmail}로 변경하려고 해요. 이메일 주소를 변경하기 위해 아래 버튼을 클릭해서 이메일 주소를 인증해 주세요.`}
      button="이메일 인증하기"
      validity="24시간"
      url={verificationUrl}
    />
  );
};

Email.PreviewProps = {
  name: '타이피',
  newEmail: 'test@typie.co',
  verificationUrl: 'https://typie.co/auth/email?code=123456',
};

export default Email;
