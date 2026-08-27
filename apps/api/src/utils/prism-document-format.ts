export const DOCUMENT_FORMAT_PATH = 'documents/README.md';

export const DOCUMENT_FORMAT_GUIDE = `# 문서 파일 형식

\`documents/\` 아래의 \`.xml\` 파일은 타이피 문서 하나를 그대로 담은 파일이에요. 아래 규칙은 저장할 때 전부 검사되고, 어긋나면 줄·열 좌표와 함께 거절돼요. 처음 고칠 때나 빈 문서에 처음부터 쓸 때 이 문서를 먼저 읽으세요.

## 1. 파일 한 장의 모양

\`\`\`text
<root dot="…" base="…" attr:layout_mode="continuous" attr:max_width="600" mod:font_size="1200" mod:font_family="Pretendard" …>
  <paragraph dot="…">첫 문단</paragraph>
  <blockquote dot="…" attr:variant="left_line">
    <paragraph dot="…">인용 안 문단</paragraph>
  </blockquote>
  <paragraph>새로 넣은 문단은 dot이 없어요</paragraph>
</root>
\`\`\`

- 파일 전체가 \`<root>\` 하나예요. XML 선언·주석·CDATA·DTD·처리 명령은 쓸 수 없어요. UTF-8, 줄 끝은 LF예요.
- 요소 이름과 속성 이름은 전부 소문자 snake_case이고, 아래 표에 있는 것만 쓸 수 있어요. HTML 이름(\`p\`·\`b\`·\`i\`·\`em\`·\`strong\`·\`u\`·\`s\`·\`del\`·\`br\`·\`a\`·\`ul\`·\`ol\`·\`li\`·\`h1\`·\`div\`·\`span\`·\`table\`의 \`tr\`·\`td\`…)은 오류예요.
- 속성 이름에는 접두가 있어요 — 맨이름은 \`dot\`과 \`base\`뿐이고, 요소의 속성은 \`attr:\`, 블록 서식은 \`mod:\`, 빈 문단의 예비 서식은 \`carry:\`예요. 인라인 서식 요소의 속성(\`value\`·\`href\`·\`text\`)만 접두가 없어요. 속성 순서는 자유예요.
- 블록만 오는 요소(\`root\`·\`blockquote\`·\`table_row\` 등) 안에서 요소 사이의 공백과 줄바꿈은 무시돼요 — 들여쓰기는 자유예요.
- 문단(\`paragraph\`)과 접기 제목(\`fold_title\`) 안의 공백은 전부 내용이에요. 여는 태그 바로 뒤의 공백 한 칸도 문서에 들어가요. 문단 안에서 줄을 바꾸지 마세요(3절).

## 2. dot과 base

- \`dot\`은 블록의 정체성이에요. 파일에 있는 \`dot\`은 그대로 두고, 값을 지어내거나 다른 블록에 옮겨 붙이지 마세요.
- 새로 넣는 블록에는 \`dot\`을 쓰지 마세요 — 저장할 때 붙어요.
- 블록을 지우려면 요소를 통째로 지우고, 옮기려면 요소를 \`dot\`째 잘라 다른 자리에 붙이세요. 그래야 코멘트와 이력이 그 블록을 따라가요.
- 블록의 종류를 바꿔야 하면(예: 문단을 인용으로) 새 요소로 감싸거나 새 요소를 만들고, 원래 블록은 \`dot\`을 유지한 채 옮기세요. \`dot\`이 있는 요소의 이름을 바꾸는 것은 새 이름의 자식 규칙에 원래 자식이 그대로 맞을 때만 돼요.
- \`<root>\`의 \`dot\`·\`base\`와 \`attr:\`·\`mod:\` 속성은 건드리지 마세요. 새 문서에 처음부터 쓸 때도 \`<root …>\` 여는 태그는 파일에 있던 그대로 두고 그 안만 채우세요.
- \`page_break\`·\`hard_break\`·\`tab\`(빈 요소)과 인라인 서식 요소에는 \`dot\`이 없어요.

## 3. 글자·이스케이프

- 문단 안 글자 데이터에서 \`<\`는 \`&lt;\`, \`&\`는 \`&amp;\`로 써야 해요. \`>\`와 따옴표는 그대로 써도 돼요. 속성값 안에서는 \`"\`를 \`&quot;\`로 써요.
- 그 밖에 인정되는 참조는 \`&gt;\`·\`&apos;\`·\`&#10004;\`·\`&#x2714;\` 형태뿐이에요. \`&nbsp;\` 같은 HTML 엔티티는 오류예요.
- 문단 안에 줄바꿈 문자(\`\\n\`·\`\\r\`)와 탭 문자를 넣을 수 없어요 — 줄바꿈은 \`<hard_break/>\`, 탭은 \`<tab/>\`으로 써요. 새 문단이 필요하면 \`<paragraph>\`를 하나 더 만드세요.
- 이모지와 모든 유니코드 글자는 그대로 써도 돼요. 제어 문자는 안 돼요.

## 4. 블록 요소 전수

\`+\`는 하나 이상, \`*\`는 없어도 됨, \`?\`는 많아야 하나예요. "블록들"은 \`paragraph\`·\`image\`·\`file\`·\`embed\`·\`archived\`·\`blockquote\`·\`callout\`·\`bullet_list\`·\`ordered_list\`·\`horizontal_rule\`·\`fold\`·\`table\` 열두 가지예요.

| 요소 | 자식 규칙 | \`attr:\` 속성 | \`mod:\` 블록 서식 | 새로 만들기 |
|---|---|---|---|---|
| \`root\` | 블록들\`*\` 다음에 \`paragraph\` 하나 — **마지막 자식은 반드시 문단**이에요 | \`layout_mode\`(\`continuous\`이면 \`max_width\`, \`paginated\`면 \`page_width\`·\`page_height\`·\`page_margin_top\`·\`page_margin_bottom\`·\`page_margin_left\`·\`page_margin_right\`, px) — 건드리지 마세요 | \`font_size\`·\`font_family\`·\`font_weight\`·\`letter_spacing\`·\`line_height\`·\`block_gap\`·\`paragraph_indent\`·\`alignment\` — 문서 기본값, 건드리지 마세요 | 불가(파일에 이미 있어요) |
| \`paragraph\` | 글자, 인라인 서식 요소(5절), \`<hard_break/>\`, \`<tab/>\` 섞어서 \`*\`; \`root\` 바로 아래 문단만 맨 끝에 \`<page_break/>\` \`?\` | 없음 | \`line_height\`·\`alignment\`; \`root\` 바로 아래 문단만 \`paragraph_indent\` | 가능 |
| \`blockquote\` | (\`paragraph\` \\| \`bullet_list\` \\| \`ordered_list\`)\`+\` | \`variant\`: \`left_line\`(기본)·\`left_quote\`·\`message_sent\`·\`message_received\` | 없음 | 가능 |
| \`callout\` | (\`paragraph\` \\| \`bullet_list\` \\| \`ordered_list\`)\`+\` | \`variant\`: \`info\`(기본)·\`success\`·\`warning\`·\`danger\` | 없음 | 가능 |
| \`bullet_list\` | \`list_item\`\`+\` | 없음 | 없음 | 가능 |
| \`ordered_list\` | \`list_item\`\`+\` | 없음 | 없음 | 가능 |
| \`list_item\` | \`paragraph\` 하나 먼저, 그 뒤에 (\`paragraph\` \\| \`bullet_list\` \\| \`ordered_list\`)\`*\` — 목록 안 목록은 \`list_item\` 안에 넣어요 | 없음 | 없음 | 가능 |
| \`fold\` | \`fold_title\` 하나, 그다음 \`fold_content\` 하나 — 정확히 이 순서로 둘 | 없음 | 없음 | 가능 |
| \`fold_title\` | 글자만 — 인라인 서식·\`hard_break\`·\`tab\` 불가 | 없음 | 없음 | \`fold\`와 함께 |
| \`fold_content\` | 블록들\`+\` | 없음 | 없음 | \`fold\`와 함께 |
| \`table\` | \`table_row\`\`+\` — 모든 행의 셀 수가 같아야 해요. 표 안 어디에도(셀 안 접기 속까지) 표를 넣을 수 없어요 | \`border_style\`: \`solid\`(기본)·\`dashed\`·\`dotted\`·\`none\`; \`proportion\`: 본문 폭 대비 백분율 정수(기본 100) | \`alignment\` | 가능 |
| \`table_row\` | \`table_cell\`\`+\` | 없음 | 없음 | \`table\`과 함께 |
| \`table_cell\` | 블록들 중 \`table\`을 뺀 열한 가지\`+\` — 빈 셀은 \`<paragraph/>\` 하나를 넣어요 | \`col_width\`: px 정수(없으면 자동) | \`background_color\`: 셀 배경색, 색 이름(6절) | \`table\`과 함께 |
| \`horizontal_rule\` | 없음(\`<horizontal_rule/>\`) | \`variant\`: \`line\`(기본)·\`dashed_line\`·\`circle_line\`·\`diamond_line\`·\`circle\`·\`diamond\`·\`three_circles\`·\`three_diamonds\`·\`zigzag\` | 없음 | 가능 |
| \`image\` | 없음 | \`id\`(그대로), \`proportion\`: 백분율 정수 | \`alignment\` | 불가 — 있는 것을 옮기거나 지우거나 \`proportion\`·\`alignment\`만 바꿀 수 있어요 |
| \`file\`, \`embed\` | 없음 | \`id\`(그대로) | 없음 | 불가 — 옮기거나 지우기만 |
| \`archived\`, \`unknown\` | 없음 | \`archived\`의 \`id\`(그대로) | 없음 | 불가 — \`dot\`과 함께 있는 그대로 두세요 |

빈 요소는 \`<horizontal_rule/>\`처럼 닫아 쓰거나 \`<paragraph></paragraph>\`처럼 열고 닫아도 돼요. 문단은 비어도 돼요.

## 5. 인라인 서식 요소 전수

문단 안에서 글자·\`<hard_break/>\`·\`<tab/>\`을 요소로 감싸요. 요소는 제대로 중첩해야 하고(\`<bold><italic>…</italic></bold>\`), 같은 종류를 겹치면 안쪽 값이 이겨요. \`fold_title\` 안에서는 쓸 수 없어요.

| 요소 | 속성 | 값 |
|---|---|---|
| \`bold\`, \`italic\`, \`underline\`, \`strikethrough\` | 없음 | — |
| \`font_size\` | \`value\` | pt×100 정수, 400~12800 (12pt = \`1200\`; 편집기 메뉴: 800·900·1000·1100·1200·1400·1600·1800·2000·2200·2400·3000·3600·4800·6000·7200·9600) |
| \`font_family\` | \`value\` | 글꼴 이름(빈 값 불가). 문서에 이미 쓰인 이름을 따르세요 — 기본은 \`Pretendard\`예요 |
| \`font_weight\` | \`value\` | 100~900의 100 단위 정수 (보통 \`400\`, 굵게 \`700\`) |
| \`text_color\` | \`value\` | 색 이름(6절) |
| \`background_color\` | \`value\` | 색 이름(6절 배경색). \`none\`은 값이 아니라 요소를 빼는 거예요 |
| \`letter_spacing\` | \`value\` | em×100 정수, -50~200 (편집기 메뉴: -10·-5·0·5·10·20·40) |
| \`link\` | \`href\` | URL(빈 값 불가). 글자만 감쌀 수 있고 \`hard_break\`·\`tab\`은 감쌀 수 없어요 |
| \`ruby\` | \`text\` | 읽기(빈 값 불가). 글자만 감쌀 수 있어요 |

검색은 서식 요소를 가로지르지 않는 짧은 조각으로 하세요 — \`안녕<bold>하세요</bold>\`는 "안녕하세요"로 찾히지 않아요. 안 잡히면 그 근처를 \`read\`로 창을 열어 읽으세요.

\`\`\`xml
<paragraph>보통 글자와 <bold>굵은 글자</bold>, <bold><italic>굵고 기울인 글자</italic></bold>, <font_size value="1600">큰 글자</font_size>, <text_color value="red">빨간 글자</text_color>, <link href="https://typie.co">링크</link>, <ruby text="かんじ">漢字</ruby>, 그리고<hard_break/>줄바꿈 뒤의 글자</paragraph>
\`\`\`

## 6. 값

- 정렬(\`alignment\`): \`left\`·\`center\`·\`right\`·\`justify\`.
- 색 이름(\`text_color\`): \`black\`·\`darkgray\`·\`gray\`·\`lightgray\`·\`white\`·\`red\`·\`orange\`·\`amber\`·\`yellow\`·\`lime\`·\`green\`·\`emerald\`·\`teal\`·\`cyan\`·\`sky\`·\`blue\`·\`indigo\`·\`violet\`·\`purple\`·\`fuchsia\`·\`pink\`·\`rose\`.
- 배경색 이름(\`background_color\`, 글자와 표 셀 공통): \`gray\`·\`red\`·\`orange\`·\`yellow\`·\`green\`·\`blue\`·\`purple\`.
- \`line_height\`: 퍼센트 정수, 50~400 (편집기 메뉴: 80·100·120·140·160·180·200·220).
- \`block_gap\`(\`root\`만)·\`paragraph_indent\`: ×100 정수, 0~400 (1줄·1칸 = \`100\`; 편집기 메뉴: 0·50·100·200).
- 숫자는 전부 정수예요. 단위 문자·소수점·퍼센트 기호를 붙이지 마세요.

## 7. 블록 서식(\`mod:\`)과 예비 서식(\`carry:\`)

- 블록 서식은 4절 표의 \`mod:\` 열에 있는 요소에만 붙어요: \`mod:alignment="center"\`, \`mod:line_height="200"\`, \`mod:paragraph_indent="100"\`, 표 셀의 \`mod:background_color="yellow"\`. 값의 범위는 5·6절과 같아요.
- 문단의 서식을 지우려면 \`mod:\` 속성을 빼면 돼요 — 문서 기본값(\`root\`)으로 돌아가요.
- \`carry:\`는 글자 없는 문단이 기억하는 서식이에요(\`carry:bold=""\`, \`carry:font_size="1600"\`). 파일에 있으면 그대로 두고, 새로 만들지 마세요.

## 8. 페이지 나눔

\`<page_break/>\`는 \`root\` 바로 아래 문단의 **맨 끝**에만 올 수 있어요. 문서의 마지막 문단은 \`page_break\`로 끝날 수 없어요 — 그 뒤에 문단을 하나 더 두세요.

\`\`\`xml
<paragraph>이 문단 뒤에서 쪽이 바뀌어요<page_break/></paragraph>
<paragraph>새 쪽의 첫 문단</paragraph>
\`\`\`

## 9. 예시

표 (모든 행의 셀 수가 같아야 해요):

\`\`\`xml
<table attr:border_style="solid">
  <table_row>
    <table_cell><paragraph><bold>S</bold></paragraph></table_cell>
    <table_cell><paragraph>A</paragraph></table_cell>
    <table_cell><paragraph>T</paragraph></table_cell>
  </table_row>
  <table_row>
    <table_cell mod:background_color="yellow"><paragraph>A</paragraph></table_cell>
    <table_cell><paragraph>R</paragraph></table_cell>
    <table_cell><paragraph/></table_cell>
  </table_row>
</table>
\`\`\`

목록과 목록 안 목록:

\`\`\`xml
<bullet_list>
  <list_item><paragraph>첫째</paragraph></list_item>
  <list_item>
    <paragraph>둘째</paragraph>
    <ordered_list>
      <list_item><paragraph>둘째의 하나</paragraph></list_item>
      <list_item><paragraph>둘째의 둘</paragraph></list_item>
    </ordered_list>
  </list_item>
</bullet_list>
\`\`\`

인용과 콜아웃:

\`\`\`xml
<blockquote attr:variant="left_quote">
  <paragraph>인용문</paragraph>
  <paragraph mod:alignment="right">— 출처</paragraph>
</blockquote>
<callout attr:variant="warning">
  <paragraph>주의할 점</paragraph>
</callout>
\`\`\`

접기:

\`\`\`xml
<fold>
  <fold_title>접힌 제목</fold_title>
  <fold_content>
    <paragraph>펼치면 보이는 내용</paragraph>
  </fold_content>
</fold>
\`\`\`

구분선과 문단 서식:

\`\`\`xml
<paragraph mod:alignment="center" mod:line_height="200"><font_size value="2400"><bold>제목처럼 쓴 문단</bold></font_size></paragraph>
<horizontal_rule attr:variant="three_diamonds"/>
<paragraph mod:paragraph_indent="100">들여 쓴 본문 문단. 기호는 &lt;이렇게&gt; 쓰고, &amp;도 이렇게 써요.</paragraph>
\`\`\`

## 10. 탐색과 편집 도구

파일을 통째로 읽고 문자열로 고치는 대신, 블록 단위로 보고 고치는 도구가 있어요.

- **뼈대 보기 — \`outline-document\`.** 긴 문서는 이것부터 보세요. 기본은 \`root\` 바로 아래 블록 한 줄씩(헤딩 노릇을 하는 문단·표·인용…)이고, 표 안이나 인용 안처럼 더 볼 곳만 \`under\`에 그 블록의 주소를 주어 펼치세요. \`depth\`(기본 1)는 \`under\` 기준 깊이, \`offset\`·\`limit\`(기본 200)은 행 페이징이에요. \`full: true\`를 주면 뼈대 대신 그 블록의 xml 전체를 돌려줘요. 한 줄은 \`서수 경로 <요소 dot=… 속성…> 본문 앞부분 (N자) 자식 N\` 모양이에요 — \`자식 N\`은 depth 밖이라 접힌 자식 수예요.
- **주소 두 가지.** \`dot\`(파일에 있는 그대로, 블록이 옮겨져도 변하지 않아요)과 **서수 경로** \`3.1.2\`(루트의 3번째 블록 → 그 1번째 → 그 2번째, 1부터 세요). 아직 저장하지 않아 \`dot\`이 없는 블록은 서수 경로로만 가리킬 수 있어요. 루트는 \`root\`예요. 연산 뒤에는 서수 경로가 밀리니, 결과에 실린 행의 주소를 쓰세요. 한 배치 안에서도 앞선 연산이 넣거나 지운 만큼 뒤 블록의 서수 경로가 밀려요 — 앞선 연산의 결과를 세어 다음 연산의 주소를 쓰세요.
- **블록 고치기 — \`edit-document\`.** \`ops\`에 연산을 여러 개 담으면 위에서 아래로 차례로 적용되고, 하나라도 실패하면 아무것도 바뀌지 않아요(파일 그대로). 규칙 검사는 배치가 끝난 뒤 한 번이라, 중간 단계에서는 빈 인용이나 셀 수가 맞지 않는 표가 있어도 돼요 — 끝날 때만 맞으면 돼요. 다만 주소가 없거나 넣을 수 없는 자리(문단 안에 블록 등)는 그 연산에서 바로 실패해요. 실패 문면은 \`ops[k]:\`로 어느 연산인지 말해요. 배치가 다 끝난 뒤의 규칙 위반은 어느 연산 탓인지 가릴 수 없어서 \`ops[k]:\` 대신 \`주소:\`로 그 블록을 짚어요 — 그 주소를 \`under\`에 주어 확인하세요. 연산은 다섯이에요:
  - \`insert\` — \`xml\`(블록 요소 하나 이상)을 \`at\` 위치에 넣어요.
  - \`delete\` — \`targets\`의 블록을 안의 것까지 지워요.
  - \`move\` — \`targets\`의 블록을 \`dot\`째 \`at\` 위치로 옮겨요(여럿이면 문서 순서를 지켜 한 덩어리로).
  - \`replace\` — \`target\` 블록을 \`xml\`(요소 하나)로 갈아끼워요. 새 요소에 \`dot\`을 쓰지 않으면 원래 블록의 \`dot\`을 물려받아 코멘트와 이력이 따라가요.
  - \`set\` — \`targets\`의 속성을 바꿔요. \`attrs\`는 \`key\`·\`value\` 쌍의 목록이고, 키는 파일의 이름 그대로(\`attr:border_style\`·\`mod:alignment\`·\`carry:font_size\`), \`value\`를 빼면 그 속성을 지워요.

  \`at\`은 \`before\`·\`after\`·\`first_child\`·\`last_child\` 중 하나에 주소를 준 객체예요(\`root\`는 \`first_child\`·\`last_child\`에만). 문단을 인용으로 바꾸는 것처럼 종류를 바꿀 때는 \`replace\`로 요소 이름을 바꾸지 말고, 감쌀 요소를 \`insert\`로 새로 만든 뒤 원래 블록을 \`move\`로 그 안에 넣으세요 — \`dot\`이 그대로 따라가요.
- **문장 안 표현은 지금처럼 \`edit\`로.** 블록 도구는 블록을 넣고 빼고 옮기고 속성을 바꾸는 일에 쓰고, 문장 하나를 다듬는 일은 \`edit\`가 자연스러워요(§5의 검색 요령 그대로).

표를 넣고 셀을 채우기(빈 문서의 첫 문단 \`1\` 앞에 — 문서의 마지막 블록은 문단이어야 하니 표를 끝에 둘 때는 그 뒤에 문단을 하나 두세요):

\`\`\`json
[
  { "op": "insert", "at": { "before": "1" }, "xml": "<table><table_row><table_cell><paragraph/></table_cell><table_cell><paragraph/></table_cell></table_row></table>" },
  { "op": "replace", "target": "1.1.1", "xml": "<table_cell><paragraph>왼쪽</paragraph></table_cell>" },
  { "op": "replace", "target": "1.1.2", "xml": "<table_cell><paragraph>오른쪽</paragraph></table_cell>" }
]
\`\`\`

문단을 넣고 옮기기(넷째 문단을 셋째 앞으로):

\`\`\`json
[
  { "op": "insert", "at": { "last_child": "root" }, "xml": "<paragraph>둘째</paragraph><paragraph>셋째</paragraph>" },
  { "op": "move", "targets": ["4"], "at": { "before": "3" } }
]
\`\`\`

표 테두리와 폭 바꾸기:

\`\`\`json
[
  { "op": "set", "targets": ["1"], "attrs": [ { "key": "attr:border_style", "value": "dashed" }, { "key": "attr:proportion", "value": "80" } ] }
]
\`\`\`

문단을 인용으로 감싸기(\`dot\` 유지 — 셋째 문단 \`3\` 뒤에 빈 인용을 만들고 그 안으로 옮겨요):

\`\`\`json
[
  { "op": "insert", "at": { "after": "3" }, "xml": "<blockquote attr:variant=\\"left_line\\"/>" },
  { "op": "move", "targets": ["3"], "at": { "first_child": "4" } }
]
\`\`\`

빈 문단 지우기:

\`\`\`json
[
  { "op": "delete", "targets": ["2"] }
]
\`\`\`
`;
