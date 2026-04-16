package co.typie.domain.entity

import co.typie.graphql.fragment.EntityIcon_entity
import co.typie.graphql.type.EntityType
import co.typie.icons.Lucide
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.DarkColors
import co.typie.ui.theme.LightColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame

class EntityIconPolicyTest {
  @Test
  fun `folder fallback uses folder icon and text secondary tint`() {
    val appearance =
      EntityIcon_entity(__typename = "Entity", type = EntityType.FOLDER, icon = "", iconColor = "")
        .iconAppearance(LightColors)

    assertSame(Lucide.Folder, appearance.icon)
    assertEquals(LightColors.textSecondary, appearance.tint)
  }

  @Test
  fun `document fallback uses file icon and text secondary tint`() {
    val appearance =
      EntityIcon_entity(
          __typename = "Entity",
          type = EntityType.DOCUMENT,
          icon = "",
          iconColor = "",
        )
        .iconAppearance(LightColors)

    assertSame(Lucide.File, appearance.icon)
    assertEquals(LightColors.textSecondary, appearance.tint)
  }

  @Test
  fun `entity icon appearance preserves explicit icon and color`() {
    val appearance =
      EntityIcon_entity(
          __typename = "Entity",
          type = EntityType.DOCUMENT,
          icon = "book-open",
          iconColor = "purple",
        )
        .iconAppearance(DarkColors)

    assertSame(Lucide.BookOpen, appearance.icon)
    assertEquals(AppColor.dark.brand.s200, appearance.tint)
  }
}
