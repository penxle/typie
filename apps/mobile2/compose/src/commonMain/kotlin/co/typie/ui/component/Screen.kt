package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import co.typie.ext.plus
import co.typie.ext.statusBars
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

@Composable
fun Screen(
  modifier: Modifier = Modifier,
  loading: Boolean = false,
  background: Color = AppTheme.colors.surfaceDefault,
  contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp),
  content: @Composable (contentPadding: PaddingValues) -> Unit,
) {
  val topBarState = LocalTopBarState.current
  val hasTopBar = topBarState != null && topBarState.enabled && topBarState.visible
  val statusBarTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
  val adjustedContentPadding = if (hasTopBar) {
    contentPadding + PaddingValues(top = statusBarTop + TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight + TopBarDefaults.ContentTopSpacing)
  } else {
    contentPadding
  }

  Box(
    Modifier
      .fillMaxSize()
      .background(background)
      .then(modifier),
  ) {
    Skeleton(enabled = loading) {
      content(adjustedContentPadding)
    }
  }
}
