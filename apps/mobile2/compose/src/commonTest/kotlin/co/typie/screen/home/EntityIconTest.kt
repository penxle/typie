package co.typie.screen.home

import co.typie.icons.Lucide
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.DarkColors
import co.typie.ui.theme.LightColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame

class EntityIconTest {
  @Test
  fun `supported entity icon names resolve to lucide icons`() {
    assertSame(
      Lucide.BookOpen,
      resolveEntityIconAppearance(
          iconName = "book-open",
          iconColor = "blue",
          fallbackIcon = Lucide.File,
          fallbackTint = LightColors.textMuted,
          colors = LightColors,
        )
        .icon,
    )
    assertSame(
      Lucide.House,
      resolveEntityIconAppearance(
          iconName = "home",
          iconColor = "blue",
          fallbackIcon = Lucide.File,
          fallbackTint = LightColors.textMuted,
          colors = LightColors,
        )
        .icon,
    )
    assertSame(
      Lucide.FingerprintPattern,
      resolveEntityIconAppearance(
          iconName = "fingerprint",
          iconColor = "blue",
          fallbackIcon = Lucide.File,
          fallbackTint = LightColors.textMuted,
          colors = LightColors,
        )
        .icon,
    )
    assertSame(
      Lucide.BarChartBig,
      resolveEntityIconAppearance(
          iconName = "bar-chart-2",
          iconColor = "blue",
          fallbackIcon = Lucide.File,
          fallbackTint = LightColors.textMuted,
          colors = LightColors,
        )
        .icon,
    )
    assertSame(
      Lucide.Package2,
      resolveEntityIconAppearance(
          iconName = "package",
          iconColor = "blue",
          fallbackIcon = Lucide.File,
          fallbackTint = LightColors.textMuted,
          colors = LightColors,
        )
        .icon,
    )
  }

  @Test
  fun `unsupported entity icon names fall back to caller icon`() {
    assertSame(
      Lucide.Folder,
      resolveEntityIconAppearance(
          iconName = "does-not-exist",
          iconColor = "green",
          fallbackIcon = Lucide.Folder,
          fallbackTint = LightColors.palette.purple,
          colors = LightColors,
        )
        .icon,
    )
  }

  @Test
  fun `entity icon colors resolve against active theme palette`() {
    assertEquals(
      AppColor.light.palette.blue,
      resolveEntityIconAppearance(
          iconName = "file",
          iconColor = "blue",
          fallbackIcon = Lucide.File,
          fallbackTint = LightColors.textMuted,
          colors = LightColors,
        )
        .tint,
    )
    assertEquals(
      AppColor.dark.palette.purple,
      resolveEntityIconAppearance(
          iconName = "file",
          iconColor = "purple",
          fallbackIcon = Lucide.File,
          fallbackTint = DarkColors.textMuted,
          colors = DarkColors,
        )
        .tint,
    )
  }

  @Test
  fun `unsupported entity icon colors fall back to caller tint`() {
    assertEquals(
      LightColors.palette.purple,
      resolveEntityIconAppearance(
          iconName = "folder",
          iconColor = "unknown",
          fallbackIcon = Lucide.Folder,
          fallbackTint = LightColors.palette.purple,
          colors = LightColors,
        )
        .tint,
    )
  }

  @Test
  fun `missing entity icon values fall back cleanly`() {
    val appearance =
      resolveEntityIconAppearance(
        iconName = null,
        iconColor = null,
        fallbackIcon = Lucide.Folder,
        fallbackTint = LightColors.textMuted,
        colors = LightColors,
      )

    assertSame(Lucide.Folder, appearance.icon)
    assertEquals(LightColors.textMuted, appearance.tint)
  }
}
