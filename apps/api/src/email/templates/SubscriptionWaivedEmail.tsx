import { Heading, Link, Text } from '@react-email/components';
import TypieEmail from './components/TypieEmail.tsx';

type Props = {
  userName: string;
  interval: 'MONTHLY' | 'YEARLY';
  waivedStart: string;
  waivedEnd: string;
};

const Email = ({ userName, interval, waivedStart, waivedEnd }: Props) => {
  const cycle = interval === 'YEARLY' ? '올해' : '이번 달';
  const period = interval === 'YEARLY' ? '1년' : '한 달';
  const unit = interval === 'YEARLY' ? '해' : '달';

  return (
    <TypieEmail preview="쓰지 않는 달은 구독료가 발생하지 않아요.">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">{cycle} 구독료는 0원이에요</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        {userName}님, 마지막으로 타이피와 함께 글을 쓰신 지 {period}이 지났네요.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        {waivedStart}부터 {waivedEnd} 사이 타이피를 사용하신 내역이 없어, {cycle}에는 {userName}님의 구독료 결제를 건너뛰었어요.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        타이피는 실제로 타이피를 사용하신 {unit}에만 구독료를 청구하고 있어요. 일상의 바쁨에 잠시 글을 쓰지 못했더라도, 잠시 휴식이 필요해
        글쓰기와 거리를 두고 싶어졌더라도 괜찮답니다. 타이피에 작성된 글은 계속해서 남아 있으니 원하실 때 다시 돌아와 이어서 쓰실 수 있어요.
        언제든 편하게 돌아와 주세요.
      </Text>

      <Text className="text-[14px] text-zinc-500 leading-[1.5]">
        궁금한 점이 있으시면{' '}
        <Link href="https://penxle.channel.io" className="text-zinc-500 underline">
          고객센터
        </Link>
        로 문의해 주세요.
      </Text>
    </TypieEmail>
  );
};

Email.PreviewProps = {
  userName: '유령선',
  interval: 'MONTHLY' as const,
  waivedStart: '2026년 2월 25일',
  waivedEnd: '3월 25일',
};

export default Email;
