import { Button, Heading, Text } from '@react-email/components';
import TypieEmail from './TypieEmail';

type Props = {
  preview: string;
  heading: string;
  text: string;
  button: string;
  validity: string;
  extra?: string;
  url: string;
};

const TypieLinkEmail = ({ preview, heading, text, button, validity, extra, url }: Props) => {
  return (
    <TypieEmail preview={preview}>
      <Heading className="text-[28px] font-bold text-zinc-950 mb-[20px] tracking-[-0.02em]">{heading}</Heading>

      <Text className="text-[16px] text-zinc-700 mb-[28px] leading-[1.5]">{text}</Text>

      <Button
        className="bg-zinc-950 text-white py-[10px] px-[20px] rounded-[4px] font-medium text-[15px] no-underline text-center box-border"
        href={url}
      >
        {button}
      </Button>

      <Text className="text-[14px] text-zinc-500 mt-[32px] mb-[10px] leading-[1.5]">
        버튼이 작동하지 않는다면, 아래 링크를 복사해서 브라우저에 붙여넣어 주세요:
      </Text>

      <Text className="text-[14px] text-zinc-700 font-mono bg-zinc-100 p-[12px] rounded-[4px] break-all">{url}</Text>

      <Text className="text-[14px] text-zinc-500 mt-[32px] leading-[1.5]">이 링크는 {validity} 동안 유효해요.</Text>

      {extra && <Text className="text-[14px] text-zinc-500 mt-[12px] leading-[1.5]">{extra}</Text>}
    </TypieEmail>
  );
};

export default TypieLinkEmail;
