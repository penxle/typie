package co.typie.screen.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject

data class SettingsItem(
  val label: String,
)

data class SettingsSection(
  val title: String,
  val items: List<SettingsItem>,
)

internal fun settingsSections(): List<SettingsSection> {
  return listOf(
    SettingsSection(
      title = "계정 설정",
      items = listOf(
        SettingsItem("이메일 변경"),
        SettingsItem("프로필 변경"),
        SettingsItem("비밀번호 변경"),
        SettingsItem("연결된 SNS 계정"),
      ),
    ),
    SettingsSection(
      title = "화면 설정",
      items = listOf(
        SettingsItem("테마"),
      ),
    ),
    SettingsSection(
      title = "편집 경험 설정",
      items = listOf(
        SettingsItem("에디터 설정"),
        SettingsItem("텍스트 대치"),
      ),
    ),
    SettingsSection(
      title = "스페이스",
      items = listOf(
        SettingsItem("현재 스페이스 설정"),
      ),
    ),
    SettingsSection(
      title = "이벤트 알림 설정",
      items = listOf(
        SettingsItem("이벤트 및 타이피 소식 받아보기"),
      ),
    ),
    SettingsSection(
      title = "서비스 정보",
      items = listOf(
        SettingsItem("이용약관"),
        SettingsItem("개인정보처리방침"),
        SettingsItem("사업자 정보"),
        SettingsItem("오픈소스 라이센스"),
        SettingsItem("버전 정보"),
      ),
    ),
    SettingsSection(
      title = "기타",
      items = listOf(
        SettingsItem("로그아웃"),
        SettingsItem("회원 탈퇴"),
      ),
    ),
  )
}

@Composable
fun SettingsScreen() {
  val toast = koinInject<Toast>()
  val scrollState = rememberScrollState()
  val sections = remember { settingsSections() }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    background = AppTheme.colors.surfaceBase,
  ) { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .verticalScroll(scrollState)
        .padding(contentPadding)
        .navigationBarsPadding(),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        "설정",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      sections.forEach { section ->
        SettingsSectionCard(
          section = section,
          onItemClick = {
            toast.show(ToastType.Notification, "준비 중인 기능이에요.")
          },
        )
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun SettingsSectionCard(
  section: SettingsSection,
  onItemClick: (SettingsItem) -> Unit,
) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    SectionTitle(
      section.title,
      modifier = Modifier.padding(top = 4.dp),
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column {
        section.items.forEachIndexed { index, item ->
          SettingsRow(
            item = item,
            onClick = { onItemClick(item) },
          )

          if (index < section.items.lastIndex) {
            CardDivider()
          }
        }
      }
    }
  }
}

@Composable
private fun SettingsRow(
  item: SettingsItem,
  onClick: () -> Unit,
) {
  CardRow(
    onClick = onClick,
  ) {
    Text(
      text = item.label,
      style = AppTheme.typography.label,
      modifier = Modifier.weight(1f),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    Spacer(Modifier.size(4.dp))

    Icon(
      icon = Lucide.ChevronRight,
      modifier = Modifier.size(16.dp),
      tint = AppTheme.colors.textTertiary,
    )
  }
}
