import ActivityIcon from '~icons/lucide/activity';
import CalendarClockIcon from '~icons/lucide/calendar-clock';
import ChartColumnIcon from '~icons/lucide/chart-column';
import Columns2Icon from '~icons/lucide/columns-2';
import FocusIcon from '~icons/lucide/focus';
import GlobeIcon from '~icons/lucide/globe';
import LayoutTemplateIcon from '~icons/lucide/layout-template';
import LockIcon from '~icons/lucide/lock';
import MessageSquareIcon from '~icons/lucide/message-square';
import NewspaperIcon from '~icons/lucide/newspaper';
import NotebookPenIcon from '~icons/lucide/notebook-pen';
import PaletteIcon from '~icons/lucide/palette';
import SaveIcon from '~icons/lucide/save';
import SparklesIcon from '~icons/lucide/sparkles';
import SpellCheckIcon from '~icons/lucide/spell-check';
import TableIcon from '~icons/lucide/table';
import TargetIcon from '~icons/lucide/target';
import UploadIcon from '~icons/lucide/upload';
import UsersIcon from '~icons/lucide/users';
import WandSparklesIcon from '~icons/lucide/wand-sparkles';
import { FILL_RANGE } from './scenes/record/activity';
import RecordScene from './scenes/record/RecordScene.svelte';
import { DIALOGUE_WINDOWS } from './scenes/revise/dialogue';
import ReviseScene from './scenes/revise/ReviseScene.svelte';
import { REACTION, REACTION_SEQUENCE } from './scenes/share/share';
import ShareScene from './scenes/share/ShareScene.svelte';
import { SWEEP_RANGE } from './scenes/typeset/typeset';
import TypesetScene from './scenes/typeset/TypesetScene.svelte';
import { TYPE_SECOND_RANGE, TYPE_SPAN } from './scenes/write/device';
import WriteScene from './scenes/write/WriteScene.svelte';
import type { Component } from 'svelte';
import type { SceneProps } from './scrub';

export type Lead = { headline: readonly [string, string]; body: readonly string[] };

export type Card = { title: string; body: string; icon: Component };

export type Feature = { id: string; lead: Lead; cards: readonly Card[]; scene: Component<SceneProps>; sceneEnd: number };

export const FEATURES: readonly Feature[] = [
  {
    id: 'write',
    scene: WriteScene,
    sceneEnd: Math.min(1, TYPE_SECOND_RANGE[1] + TYPE_SPAN),
    lead: {
      headline: ['매일이든 가끔이든,', '계속 쓰는 사람들을 위한 글쓰기 앱'],
      body: [
        '집에서 쓰다가 밖에서 생각이 나면, 그때 손에 잡히는 걸 열어 이어 쓰면 돼요.',
        '웹이든 데스크톱이든 폰이든 같은 글이 실시간으로 동기화되니까 파일을 옮기거나 저장을 누를 일이 없고요.',
        '한참 손을 놓았다가 열어도 마지막으로 보던 자리 그대로라, 어디까지 썼는지 찾을 필요도 없어요.',
      ],
    },
    cards: [
      {
        title: '자동 저장',
        icon: SaveIcon,
        body: '쓰는 동안 변경된 내용이 곧바로 저장돼요. 실수로 탭을 닫아도, 인터넷 연결이 끊겨도 쓴 내용은 온전히 남아 있어요.',
      },
      {
        title: '집중',
        icon: FocusIcon,
        body: '작성 중인 줄을 강조하고 화면의 원하는 높이에 고정해 시선을 유지해요. 집중 모드를 켜면 목록도 도구도 사라지고 작성에만 몰입할 수 있어요.',
      },
      {
        title: '입력 보조',
        icon: WandSparklesIcon,
        body: '마침표 세 개를 치면 말줄임표(…)로, 하이픈 두 개를 치면 줄표(—)로 바뀌어요. 따옴표는 여는 것과 닫는 것을 알아서 구분해 넣고, 자주 쓰는 표현은 직접 규칙으로 등록할 수도 있어요.',
      },
      {
        title: '화면 분할',
        icon: Columns2Icon,
        body: '문서를 끌어다 창 가장자리에 놓으면 그 방향으로 화면이 나뉘어요. 참고할 글을 옆에 띄워두고 쓰거나 두 글을 오가며 옮겨 적을 때, 창을 바꿀 필요가 없어요.',
      },
    ],
  },
  {
    id: 'revise',
    scene: ReviseScene,
    sceneEnd: DIALOGUE_WINDOWS.at(-1)?.blocks.at(-1) ?? 1,
    lead: {
      headline: ['쓰는 것보다', '고치는 게 오래 걸리니까'],
      body: [
        '프리즘은 내 작업실을 함께 쓰는 담당 편집자예요.',
        '어디를 어떻게 고칠지 같이 이야기하다가, 정하고 나면 원고에 옮기는 일까지 맡길 수 있어요.',
        '어질러진 문서를 정리하거나 글쓰기 목표를 세우는 것처럼 글 곁의 일도요.',
      ],
    },
    cards: [
      {
        title: '프리즘 리뷰',
        icon: SparklesIcon,
        body: '원고 전체를 읽고 걸리는 자리마다 본문 옆 여백에 피드백을 남겨요. 다 읽고 나서는 이 글이 어떻게 읽혔는지, 어디부터 고치면 좋을지를 따로 정리해줘요.',
      },
      {
        title: '맞춤법 검사',
        icon: SpellCheckIcon,
        body: '바른한글(구 부산대) 맞춤법 검사기로 띄어쓰기부터 문장부호, 오용어까지 열 가지 유형을 짚어요. 왜 틀렸는지 설명과 고칠 말이 함께 나오고, 일부러 그렇게 쓴 표현은 같은 단어까지 한 번에 넘길 수 있어요.',
      },
      {
        title: '노트',
        icon: NotebookPenIcon,
        body: '글에 도움이 될 것들을 자유롭게 적어두고 원고에 연결해두세요. 색으로 갈래를 나누고, 처리한 것은 완료로 표시해 목록에서 지워갈 수 있어요.',
      },
      {
        title: '코멘트',
        icon: MessageSquareIcon,
        body: '문장을 골라 코멘트를 달면 그 자리에 스레드가 열려요. 링크를 받은 사람도 같은 자리에 의견을 남길 수 있어서, 원고를 보여주고 고쳐가는 과정을 여기서 끝낼 수 있어요.',
      },
    ],
  },
  {
    id: 'typeset',
    scene: TypesetScene,
    sceneEnd: SWEEP_RANGE[1],
    lead: {
      headline: ['글꼴부터 여백까지,', '내가 정한 모양으로'],
      body: [
        '글꼴과 글자 크기, 자간과 행간, 본문 폭, 첫 줄 들여쓰기와 문단 사이 간격까지 직접 정해요.',
        '스크롤로 이어 볼지 A4 같은 페이지로 끊어 볼지도 고를 수 있고요.',
        '정한 모양은 글을 공개했을 때 읽는 사람 화면에도 그대로 나와요.',
      ],
    },
    cards: [
      {
        title: '테마',
        icon: PaletteIcon,
        body: '라이트와 다크를 합쳐 서른 가지가 넘는 테마가 있어요. 두 벌을 정해두면 밝을 때와 어두울 때 알아서 바뀌어요.',
      },
      {
        title: '글꼴 업로드',
        icon: UploadIcon,
        body: '쓰고 싶은 글꼴이 목록에 없다면 파일을 직접 올리세요. 올린 글꼴은 내 글이라면 어디서나 이용할 수 있어요.',
      },
      {
        title: '서식 도구',
        icon: TableIcon,
        body: '표와 인용구, 접기와 강조, 이미지와 임베드를 본문에 바로 넣어요. 한자나 외국어에 독음을 다는 루비, 원하는 자리에서 장을 끊는 페이지 나눔처럼 원고에만 필요한 것들도 있어요.',
      },
      {
        title: '템플릿과 프리셋',
        icon: LayoutTemplateIcon,
        body: '쓰던 문서를 템플릿으로 바꿔두면 새 글에서 불러와 쓸 수 있어요. 글꼴과 여백처럼 매번 똑같이 맞추던 조판은 프리셋에 넣어두면 새 문서에 자동으로 적용돼요.',
      },
    ],
  },
  {
    id: 'share',
    scene: ShareScene,
    sceneEnd: REACTION.start + (REACTION_SEQUENCE.length - 1) * REACTION.step + REACTION.span,
    lead: {
      headline: ['다 썼으면,', '이제 읽힐 차례니까'],
      body: [
        '공개하면 글마다 주소가 하나 생겨요.',
        '주소를 받은 사람은 로그인도 설치도 없이 바로 읽고요.',
        '다 읽고 나면 박수나 하트 같은 이모지로 반응을 남길 수 있어요.',
      ],
    },
    cards: [
      {
        title: '공개 범위',
        icon: GlobeIcon,
        body: '쓴 글은 내가 열기 전까지 비공개예요. 공개하면 게시 페이지에 올라가고, 링크 공개로 두면 주소를 아는 사람만 볼 수 있어요.',
      },
      {
        title: '비밀번호와 연령 제한',
        icon: LockIcon,
        body: '링크를 열어두더라도 비밀번호를 걸 수 있고, 15세나 성인으로 연령 제한을 둘 수 있어요. 우클릭과 복사, 다운로드를 막는 내용 보호도 함께 켤 수 있어요.',
      },
      {
        title: '함께 편집',
        icon: UsersIcon,
        body: '둘이서 한 작품을 나눠 쓰거나, 봐줄 사람에게 원고를 직접 고치게 맡길 수 있어요. 같은 원고를 동시에 열어두고 실시간으로 함께 작업해요.',
      },
      {
        title: '게시 페이지',
        icon: NewspaperIcon,
        body: '공개한 글은 내 작업실 주소에 모여요. 주소는 원하는 이름으로 바꿀 수 있어서, 그 자체로 내 글을 모아 보여주는 자리가 돼요.',
      },
    ],
  },
  {
    id: 'record',
    scene: RecordScene,
    sceneEnd: FILL_RANGE[1],
    lead: {
      headline: ['쓴 만큼', '그대로 남으니까'],
      body: [
        '따로 세지 않아도 그날 쓴 만큼이 칸 하나로 남아요.',
        '많이 쓴 날은 진하게, 안 쓴 날은 비어 있게 지난 1년이 한 화면에 들어와요.',
        '어느 달에 많이 썼고 어디서 끊겼는지가 그대로 보이고요.',
      ],
    },
    cards: [
      {
        title: '일일 목표와 문서 목표',
        icon: TargetIcon,
        body: '하루에 얼마나 쓸지, 이 글을 몇 자까지 쓸지 정해두세요. 목록에서 글마다 얼마나 찼는지 보이고, 마감일까지 넣으면 하루에 얼마씩 써야 하는지도 나와요.',
      },
      {
        title: '글자 수와 변화량',
        icon: ActivityIcon,
        body: '공백 포함, 공백 미포함, 공백/부호 미포함 글자 수를 골라 봐요. 오늘 입력한 글자와 지운 글자도 따로 나와서, 매일 얼마나 쓰고 고쳤는지 알 수 있어요.',
      },
      {
        title: '통계',
        icon: ChartColumnIcon,
        body: '총 글자와 총 문서, 활동일과 연속 기록이 한 화면에 모여요. 무슨 요일에 가장 많이 쓰는지도 나오고, 그 화면을 이미지로 저장할 수 있어요.',
      },
      {
        title: '타임라인',
        icon: CalendarClockIcon,
        body: '문서의 지난 버전이 날짜와 시각과 함께 쌓이고, 원하는 버전으로 되돌릴 수 있어요. 되돌리지 않고 그 시점 원고를 열어보기만 할 수도 있어서, 지웠던 대목을 다시 찾을 때 쓸 수 있어요.',
      },
    ],
  },
];
