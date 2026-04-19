package co.typie.domain.subscription

import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

enum class PlanUpgradeBenefit(val icon: IconData, val title: String, val description: String) {
  CustomSpaceAddress(Lucide.Globe, "나만의 스페이스 주소", "기억하기 쉬운 주소로 공유해요"),
  CustomFontUpload(Lucide.Type, "커스텀 폰트 업로드", "나만의 글꼴로 공간에 개성을 입혀요"),
  UnlimitedCharacters(Lucide.BookOpenText, "무제한 글자 수", "길어도 얼마든지 쓸 수 있어요"),
  UnlimitedFileUpload(Lucide.Images, "무제한 파일 업로드", "이미지와 첨부 용량 제한 없이"),
  SpellCheck(Lucide.SpellCheck, "맞춤법 검사", "실시간으로 맞춤법을 확인해요"),
  BetaAccess(Lucide.FlaskConical, "베타 기능 우선 접근", "신기능을 가장 먼저 써볼 수 있어요"),
  PrioritySupport(Lucide.Headset, "문제 발생 시 우선 지원", "도움이 필요할 때 우선으로 응대해요"),
  DiscordCommunity(Lucide.Sprout, "디스코드 커뮤니티 참여", "다른 사용자들과 함께 쓰는 방법을 나눠요"),
}
