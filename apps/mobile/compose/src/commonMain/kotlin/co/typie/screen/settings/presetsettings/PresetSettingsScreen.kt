package co.typie.screen.settings.presetsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.preview.EditorPreview
import co.typie.editor.runtime.EditorRuntime
import co.typie.ext.imePadding
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.editorsettings.EditorSettingsFontSection
import co.typie.ui.component.editorsettings.EditorSettingsLayoutSection
import co.typie.ui.component.editorsettings.EditorSettingsSectionDivider
import co.typie.ui.component.editorsettings.EditorSettingsSpacingSection
import co.typie.ui.component.editorsettings.EditorStyleSettings
import co.typie.ui.component.editorsettings.toEditorModifiers
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun PresetSettingsScreen() {
  val model = viewModel { PresetSettingsViewModel() }
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val toast = LocalToast.current
  val sheet = LocalSheet.current
  val nav = Nav.current

  suspend fun save(preset: Preset) {
    if (!SubscriptionService.gate(sheet, nav, GatedAction.PresetSettings)) return
    model.updatePreset(preset).withDefaultExceptionHandler(toast)
  }

  suspend fun saveStyle(style: EditorStyleSettings) {
    save(model.preset.withStyle(style))
  }

  suspend fun saveLayout(layout: LayoutMode) {
    save(model.preset.copy(layout = layout.toPresetPageLayout()))
  }

  ProvideTopBar(
    center = { Text("프리셋", style = AppTheme.typography.title) },
    trailing = {
      PresetMenu(
        onReset = {
          scope.launch {
            if (SubscriptionService.gate(sheet, nav, GatedAction.PresetSettings)) {
              model.resetPreset().withDefaultExceptionHandler(toast)
            }
          }
        }
      )
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  val layout = model.preset.layout.toLayoutMode()
  val screenBackground =
    when (layout) {
      is LayoutMode.Continuous -> AppTheme.colors.surfaceDefault
      is LayoutMode.Paginated -> AppTheme.colors.surfaceInset
    }

  Screen(
    loadable = model.query,
    background = screenBackground,
    contentPadding = PaddingValues.Zero,
  ) { contentPadding ->
    val colors = AppTheme.colors
    val layoutDirection = LocalLayoutDirection.current
    val topBarClearance = contentPadding.calculateTopPadding()
    val previewHeight = 200.dp
    val previewContainerHeight = topBarClearance + previewHeight
    val previewShape = RoundedCornerShape(bottomStart = AppShapes.xl, bottomEnd = AppShapes.xl)
    val style = model.preset.toEditorStyleSettings()
    val previewRuntime = remember { EditorRuntime(uiScope = scope) }

    Box(
      modifier =
        Modifier.fillMaxSize()
          .imePadding()
          .padding(
            start = contentPadding.calculateStartPadding(layoutDirection),
            end = contentPadding.calculateEndPadding(layoutDirection),
          )
    ) {
      Column(
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(scrollState)
            .background(colors.surfaceDefault)
            .padding(
              top = previewContainerHeight + 12.dp,
              bottom = contentPadding.calculateBottomPadding(),
            )
            .padding(AppTheme.spacings.scrollBottomPadding)
      ) {
        EditorSettingsFontSection(
          style = style,
          fontFamilies = model.fontFamilies,
          sheet = sheet,
          onStyleChange = ::saveStyle,
        )

        EditorSettingsSectionDivider()

        EditorSettingsSpacingSection(style = style, onStyleChange = ::saveStyle)

        EditorSettingsSectionDivider()

        EditorSettingsLayoutSection(layout = layout, sheet = sheet, onLayoutChange = ::saveLayout)
      }

      Box(modifier = Modifier.fillMaxWidth()) {
        EditorPreview(
          layoutMode = layout,
          runtime = previewRuntime,
          modifier = Modifier.fillMaxWidth().height(previewContainerHeight).zIndex(1f),
          shape = previewShape,
          contentTopPadding = topBarClearance,
          modifiers = style.toEditorModifiers(),
        )

        Box(
          modifier =
            Modifier.fillMaxWidth()
              .height(16.dp + AppShapes.xl / 2)
              .offset(y = previewContainerHeight - AppShapes.xl / 2)
              .background(
                Brush.verticalGradient(
                  colors = listOf(colors.surfaceInset, colors.surfaceInset.copy(alpha = 0f))
                )
              )
        )
      }
    }
  }
}

@Composable
private fun PresetMenu(onReset: () -> Unit) {
  val scope = rememberCoroutineScope()
  val dialog = LocalDialog.current
  val colors = AppTheme.colors

  PopoverMenu(anchor = { TopBarButton(icon = Lucide.Ellipsis) }) {
    item(icon = Lucide.RotateCcw, label = "프리셋 초기화", color = colors.danger) {
      scope.launch {
        val result =
          dialog.confirm(
            title = "프리셋 초기화",
            message = "모든 프리셋 설정을 기본값으로 되돌려요. 이 작업은 되돌릴 수 없어요.",
            confirmText = "초기화",
            confirmIsDestructive = true,
          )
        if (result is DialogResult.Resolved) {
          onReset()
        }
      }
    }
  }
}

private fun Preset.toEditorStyleSettings(): EditorStyleSettings =
  EditorStyleSettings(
    fontFamily = fontFamily,
    fontSize = fontSize,
    fontWeight = fontWeight,
    letterSpacing = letterSpacing,
    lineHeight = lineHeight,
    paragraphIndent = paragraphIndent,
    blockGap = blockGap,
  )

private fun Preset.withStyle(style: EditorStyleSettings): Preset =
  copy(
    fontFamily = style.fontFamily,
    fontSize = style.fontSize,
    fontWeight = style.fontWeight,
    letterSpacing = style.letterSpacing,
    lineHeight = style.lineHeight,
    paragraphIndent = style.paragraphIndent,
    blockGap = style.blockGap,
  )
