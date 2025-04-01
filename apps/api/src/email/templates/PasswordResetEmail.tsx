import TypieLinkEmail from './components/TypieLinkEmail';

type Props = {
  resetUrl: string;
};

const Email = ({ resetUrl }: Props) => {
  return (
    <TypieLinkEmail
      preview="비밀번호를 재설정해 주세요"
      heading="비밀번호를 재설정해 주세요"
      text="비밀번호 재설정을 요청하셨어요. 아래 버튼을 클릭해서 새로운 비밀번호를 설정해 주세요."
      button="비밀번호 재설정하기"
      validity="1시간"
      url={resetUrl}
      extra="만약 비밀번호 재설정을 요청하지 않으셨다면, 이 이메일은 무시하셔도 돼요."
    />
  );
};

Email.PreviewProps = {
  resetUrl: 'https://typie.co/auth/reset-password?code=123456',
};

export default Email;
