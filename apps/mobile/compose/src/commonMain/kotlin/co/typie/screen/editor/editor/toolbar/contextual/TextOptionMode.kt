package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorValues
import co.typie.editor.ffi.Alignment as FfiAlignment
import co.typie.editor.ffi.BackgroundColorValue
import co.typie.editor.ffi.TextColorValue
import co.typie.editor.ffi.Tri
import co.typie.graphql.fragment.EditorSettingsFontFamily_family
import co.typie.graphql.type.FontFamilySource
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.ToolbarBorderWidth
import co.typie.screen.editor.editor.toolbar.ToolbarButtonShape
import co.typie.screen.editor.editor.toolbar.ToolbarButtonSize
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs

internal enum class TextOptionMode {
  TextColor,
  BackgroundColor,
  FontFamily,
  FontWeight,
  FontSize,
  Alignment,
  LineHeight,
  LetterSpacing,
}

internal inline fun <T, R> Tri<T>?.uniformValue(transform: (T) -> R): R? =
  when (this) {
    is Tri.Uniform -> transform(value)
    else -> null
  }

internal fun Tri<TextColorValue>?.textColorCurrentValue(): String? =
  when (this) {
    is Tri.Uniform -> value.value
    Tri.Absent -> "black"
    Tri.Mixed,
    null -> null
  }

internal fun Tri<BackgroundColorValue>?.backgroundColorCurrentValue(): String? =
  when (this) {
    is Tri.Uniform -> value.value
    Tri.Absent -> "none"
    Tri.Mixed,
    null -> null
  }

internal fun Tri<*>?.hasUniformValue(): Boolean = this is Tri.Uniform<*>

internal val EditorSettingsFontFamily_family.isSelectableToolbarFontFamily: Boolean
  get() =
    state == FontFamilyState.ACTIVE &&
      source != FontFamilySource.FALLBACK &&
      activeToolbarFonts.isNotEmpty()

internal val EditorSettingsFontFamily_family.activeToolbarFonts:
  List<EditorSettingsFontFamily_family.Font>
  get() = fonts.filter { it.state == FontState.ACTIVE }

internal val EditorSettingsFontFamily_family.representativeToolbarFont:
  EditorSettingsFontFamily_family.Font?
  get() = activeToolbarFonts.reduceOrNull { previous, current ->
    val previousDiff = abs(previous.weight - 400)
    val currentDiff = abs(current.weight - 400)
    when {
      currentDiff < previousDiff -> current
      currentDiff == previousDiff && current.weight > previous.weight -> current
      else -> previous
    }
  }

internal fun toolbarFontWeightLabel(
  weight: Int,
  subfamilyDisplayName: String? = null,
  available: Boolean = true,
): String =
  if (!available) "(알 수 없는 굵기)"
  else
    EditorValues.fontWeight.firstOrNull { it.value == weight }?.label
      ?: subfamilyDisplayName?.let { "$it ($weight)" }
      ?: weight.toString()

internal fun formatToolbarPointValue(value: Int): String {
  val whole = value / 100
  val fraction = abs(value % 100)
  if (fraction == 0) return whole.toString()
  return "$whole.${fraction.toString().padStart(2, '0')}".trimEnd('0').trimEnd('.')
}

internal fun toolbarAlignmentIcon(value: FfiAlignment?): IconData =
  when (value) {
    FfiAlignment.Center -> Lucide.AlignCenter
    FfiAlignment.Right -> Lucide.AlignRight
    FfiAlignment.Justify -> Lucide.AlignJustify
    FfiAlignment.Left,
    null -> Lucide.AlignLeft
  }

internal fun toolbarAlignmentLabel(value: FfiAlignment): String =
  when (value) {
    FfiAlignment.Left -> "왼쪽"
    FfiAlignment.Center -> "가운데"
    FfiAlignment.Right -> "오른쪽"
    FfiAlignment.Justify -> "양쪽"
  }

@Composable
internal fun TextToolbarSwatchButton(
  color: Color?,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  selected: Boolean = false,
  swatchShape: Shape = ToolbarButtonShape,
  showSlash: Boolean = false,
) {
  val interactionSource = remember { MutableInteractionSource() }
  val bringIntoViewRequester = remember { BringIntoViewRequester() }
  val activeRingColor =
    if (color == null || color == Color.White) AppTheme.colors.borderDefault else color
  val ringColor = if (selected) activeRingColor else Color.Transparent

  LaunchedEffect(selected) {
    if (selected) {
      bringIntoViewRequester.bringIntoView()
    }
  }

  Box(
    modifier =
      modifier
        .size(ToolbarButtonSize)
        .bringIntoViewRequester(bringIntoViewRequester)
        .focusProperties { canFocus = false }
        .semantics {
          this.contentDescription = contentDescription
          role = Role.Button
        }
        .clip(ToolbarButtonShape)
        .clickable(interactionSource = interactionSource, indication = null, onClick = onClick),
    contentAlignment = Alignment.Center,
  ) {
    Box(
      modifier = Modifier.size(26.dp).border(2.dp, ringColor, swatchShape),
      contentAlignment = Alignment.Center,
    ) {
      Box(
        modifier =
          Modifier.size(20.dp)
            .clip(swatchShape)
            .then(color?.let { Modifier.background(it, swatchShape) } ?: Modifier)
            .border(ToolbarBorderWidth, AppTheme.colors.borderDefault, swatchShape),
        contentAlignment = Alignment.Center,
      ) {
        if (showSlash) {
          Icon(
            icon = Lucide.Slash,
            contentDescription = null,
            modifier = Modifier.size(14.dp),
            tint = AppTheme.colors.textMuted,
          )
        }
      }
    }
  }
}

internal val TextBackgroundSwatchShape: Shape = AppShapes.rounded(AppShapes.sm)
