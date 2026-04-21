package co.typie.domain.entity

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.graphql.fragment.EntityIcon_entity
import co.typie.graphql.type.EntityType
import co.typie.icons.Lucide
import co.typie.ui.EntityIconAppearance
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppColors
import co.typie.ui.theme.AppTheme

fun EntityIcon_entity.iconAppearance(colors: AppColors): EntityIconAppearance {
  return resolveEntityIconAppearance(
    iconName = icon,
    iconColor = iconColor,
    fallbackIcon = if (type == EntityType.FOLDER) Lucide.Folder else Lucide.File,
    fallbackTint = colors.textMuted,
    colors = colors,
  )
}

val EntityIcon_entity.iconAppearance: EntityIconAppearance
  @Composable get() = iconAppearance(colors = AppTheme.colors)

@Composable
fun EntityIcon(entity: EntityIcon_entity, modifier: Modifier = Modifier) {
  val iconAppearance = entity.iconAppearance

  Icon(icon = iconAppearance.icon, tint = iconAppearance.tint, modifier = modifier)
}
