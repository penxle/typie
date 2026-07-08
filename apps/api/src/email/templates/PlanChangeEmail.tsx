import { Heading, Hr, Link, Text } from 'react-email';
import TypieEmail from './components/TypieEmail.tsx';

export default function Email() {
  return (
    <TypieEmail preview="더 나은 서비스 제공을 위해 2026년 7월 1일부터 플랜 및 구독 요금제가 변경됩니다">
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em] leading-[1.3]">
        타이피 플랜 및 구독 요금제가 변경됩니다
      </Heading>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        안녕하세요, 타이피 팀입니다.
        <br />더 나은 서비스 제공을 위해 2026년 7월 1일부터 타이피 플랜 구성 및 구독 요금제에 다음 변화가 생길 예정입니다.
      </Text>

      <Hr className="border-zinc-200 my-[28px]" />

      <Text className="text-[15px] text-zinc-700 mb-[14px] leading-[1.5]">
        <span className="text-zinc-400 line-through">타이피 BASIC ACCESS (무료)</span>
        <span className="text-zinc-400"> → </span>
        <strong className="text-zinc-950">폐지</strong>
      </Text>
      <Text className="text-[15px] text-zinc-700 mb-[14px] leading-[1.5]">
        타이피 FULL ACCESS (<span className="text-zinc-400 line-through">월 4,900원</span>
        <span className="text-zinc-400"> → </span>
        <strong className="text-zinc-950">월 2,900원</strong>)
      </Text>
      <Text className="text-[15px] text-zinc-700 leading-[1.5]">
        타이피 TRIAL ACCESS (2주)
        <span className="text-zinc-400"> → </span>
        <strong className="text-zinc-950">변화 없음</strong>
      </Text>

      <Hr className="border-zinc-200 my-[28px]" />

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        타이피 FULL ACCESS 혜택은 기존과 그대로 유지됩니다. 기존 타이피 FULL ACCESS를 구독하고 계실 경우 별도로 취하셔야 할 조치는 없으며,
        기존 결제 주기에 따라 2026년 7월 1일 이후 인하된 요금으로 자동으로 결제됩니다.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[12px] leading-[1.5]">
        타이피 BASIC ACCESS가 폐지됨에 따라 다음 변화가 추가적으로 있을 예정입니다.
      </Text>

      <ol className="text-[16px] text-zinc-700 leading-[1.5] pl-[24px] mt-0 mb-[20px]">
        <li className="mb-[8px]">
          기존 타이피 BASIC ACCESS 이용자는 2026년 7월 1일 00시부터 자동으로 2주간의 타이피 TRIAL ACCESS가 활성화됩니다.
        </li>
        <li>2026년 7월 1일 00시 이후 가입자는 가입 즉시 자동으로 2주간의 타이피 TRIAL ACCESS가 활성화됩니다.</li>
      </ol>

      <Text className="text-[16px] text-zinc-700 mb-[20px] leading-[1.5]">
        위 요금은 웹에서의 월 결제 기준이며, 연 결제시 기존과 같이 2달 무료 혜택이 제공돼 연 29,000원에 이용하실 수 있습니다. 앱에서의 결제
        시 스토어 수수료에 따른 추가금이 발생할 수 있습니다.
      </Text>

      <Text className="text-[16px] text-zinc-700 mb-[28px] leading-[1.5]">
        해당 변경은 2026년 7월 1일 00시 (한국 시간)부터 적용됩니다.
        <br />더 많은 이용자분들께 지속 가능한 서비스를 제공하기 위한 변경으로, 보다 나은 서비스로 보답하겠습니다. 감사합니다.
        <br />
        <br />- 타이피 팀 드림
      </Text>

      <Hr className="border-zinc-200 my-[28px]" />

      <Text className="text-[15px] font-semibold text-zinc-950 mb-[16px]">※ 자주 묻는 질문</Text>

      <Text className="text-[14px] font-semibold text-zinc-900 mb-[6px] leading-[1.5]">
        Q. 기존 타이피 BASIC ACCESS를 사용중인 이용자입니다. 만약 2주간의 트라이얼 이후 결제하지 않으면 기존 글은 어떻게 되나요?
      </Text>
      <Text className="text-[14px] text-zinc-700 mb-[20px] leading-[1.5]">
        A. 모든 글이 읽기 전용 상태로 전환되어 보존됩니다. 기존 공유된 글과 링크 또한 계속해서 접근하실 수 있으나, 새로운 글을 생성하거나
        기존 글을 편집하실 수는 없습니다.
      </Text>

      <Text className="text-[14px] font-semibold text-zinc-900 mb-[6px] leading-[1.5]">
        Q. 타이피 FULL ACCESS를 연간 결제중인 이용자입니다. 가격이 인하되면 기존 결제건에 대한 보상이나 환불이 이루어지나요?
      </Text>
      <Text className="text-[14px] text-zinc-700 mb-[20px] leading-[1.5]">
        A. 기존 연간 구독자에 대한 별도의 자동 환불은 진행되지 않습니다. 다만 필요하실 경우 7월 1일 이후 고객센터를 통해 기존 구독을 일할
        환불받으신 뒤 인하된 가격에 재구독하실 수 있습니다.
      </Text>

      <Text className="text-[14px] font-semibold text-zinc-900 mb-[6px] leading-[1.5]">
        Q. 이후 또 가격 인하나, 플랜 변경 예정이 있나요?
      </Text>
      <Text className="text-[14px] text-zinc-700 mb-[28px] leading-[1.5]">
        A. 타이피 FULL ACCESS 플랜의 추가적인 가격 인하 예정은 없습니다. 다만 추후 기능 추가에 따라 타이피 FULL ACCESS의 상위 플랜, 혹은
        사용량/크레딧 기반의 요금제가 추가될 가능성은 열어두고 있습니다.
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
}
