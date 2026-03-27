package co.typie.screen.settings

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import co.typie.icons.Lucide
import co.typie.route.Route

class SettingsModelsTest {
  @Test
  fun `settingsSections hides developer section by default`() {
    val titles = settingsSections()
      .map { it.title }

    assertFalse(titles.contains("개발자"))
  }

  @Test
  fun `settingsSections shows developer section when enabled`() {
    val developerItem = settingsSections(devModeEnabled = true)
      .first { it.title == "개발자" }
      .items
      .single()

    assertEquals("개발자 모드", developerItem.label)
    assertEquals(SettingsItemAction.DeveloperMode, developerItem.action)
  }

  @Test
  fun `settingsSections configures logout as dedicated action`() {
    val logoutItem = settingsSections()
      .first { it.title == "기타" }
      .items
      .first { it.label == "로그아웃" }

    val action = assertNotNull(logoutItem.action)

    assertEquals(SettingsItemAction.Logout, action)
  }

  @Test
  fun `confirmSettingsLogout performs logout before dismiss`() = runTest {
    val events = mutableListOf<String>()

    confirmSettingsLogout(
      onDismiss = { events += "dismiss" },
      onLogout = { events += "logout" },
    )

    assertEquals(listOf("logout", "dismiss"), events)
  }

  @Test
  fun `settingsSections configures service info external links`() {
    val itemsByLabel = settingsSections()
      .first { it.title == "서비스 정보" }
      .items
      .associateBy { it.label }

    assertEquals("https://typie.co/legal/terms", itemsByLabel.getValue("이용약관").externalUrl)
    assertEquals("https://typie.co/legal/privacy", itemsByLabel.getValue("개인정보처리방침").externalUrl)
    assertEquals("https://www.ftc.go.kr/bizCommPop.do?wrkr_no=6108803078", itemsByLabel.getValue("사업자 정보").externalUrl)
    assertSame(Route.OssLicenses, itemsByLabel.getValue("오픈소스 라이센스").route)
    assertEquals(SettingsItemAction.VersionInfo, itemsByLabel.getValue("버전 정보").action)
  }

  @Test
  fun `settingsTrailingIcon uses external link icon for external urls`() {
    assertSame(Lucide.ExternalLink, settingsTrailingIcon(SettingsItem("이용약관", externalUrl = "https://typie.co/legal/terms")))
    assertSame(Lucide.ChevronRight, settingsTrailingIcon(SettingsItem("프로필", route = co.typie.route.Route.ProfileSettings)))
  }

  @Test
  fun `settingsVersionTapResult enables developer mode on seventh tap`() {
    val result = settingsVersionTapResult(
      devModeEnabled = false,
      tapCount = 6,
    )

    assertEquals(0, result.nextTapCount)
    assertTrue(result.enableDeveloperMode)
    assertNotNull(result.message)
  }

  @Test
  fun `settingsVersionTapResult shows remaining count from fourth tap`() {
    val result = settingsVersionTapResult(
      devModeEnabled = false,
      tapCount = 3,
    )

    assertEquals(4, result.nextTapCount)
    assertFalse(result.enableDeveloperMode)
    assertNotNull(result.message)
  }

  @Test
  fun `settingsVersionTapResult short circuits when already enabled`() {
    val result = settingsVersionTapResult(
      devModeEnabled = true,
      tapCount = 2,
    )

    assertEquals(0, result.nextTapCount)
    assertFalse(result.enableDeveloperMode)
    assertNotNull(result.message)
  }
}
