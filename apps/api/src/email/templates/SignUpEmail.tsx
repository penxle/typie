import TypieLinkEmail from './components/TypieLinkEmail';

type Props = {
  verificationUrl: string;
};

const Email = ({ verificationUrl }: Props) => {
  return (
    <TypieLinkEmail
      preview="이메일 주소를 인증해 주세요"
      heading="이메일 인증이 필요해요"
      text="회원가입을 완료하고 서비스를 시작하기 위해 아래 버튼을 클릭해서 이메일 주소를 인증해 주세요."
      button="이메일 인증하기"
      validity="24시간"
      url={verificationUrl}
    />
  );
};

Email.PreviewProps = {
  verificationUrl: 'https://auth.typie.co/email?code=123456',
};

export default Email;
