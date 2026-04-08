package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import co.typie.ext.imeOrNavigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ext.navigationBars
import co.typie.ext.navigationBarsPadding
import co.typie.ext.plus
import co.typie.ext.statusBars
import co.typie.ext.verticalScroll
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.state.rememberScrollState
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

@Composable
private fun BaseScreen(
  modifier: Modifier = Modifier,
  loading: Boolean = false,
  background: Color = AppTheme.colors.surfaceDefault,
  contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp),
  responsive: Boolean = true,
  responsiveMaxWidth: Dp = ResponsiveContainerDefaults.MaxWidth,
  primaryScrollableState: ScrollableState? = null,
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
    val contentModifier = Modifier.fillMaxSize()

    val contentContainer: @Composable (@Composable () -> Unit) -> Unit = { innerContent ->
      if (responsive) {
        ResponsiveContainer(
          modifier = contentModifier,
          contentMaxWidth = responsiveMaxWidth,
          primaryScrollableState = primaryScrollableState,
          content = innerContent,
        )
      } else {
        Box(contentModifier) {
          innerContent()
        }
      }
    }

    contentContainer {
      Skeleton(enabled = loading) {
        content(adjustedContentPadding)
      }
    }
  }
}

@Composable
fun Screen(
  modifier: Modifier = Modifier,
  scrollState: ScrollState? = null,
  primaryScrollableState: ScrollableState? = null,
  loading: Boolean = false,
  background: Color = AppTheme.colors.surfaceDefault,
  responsive: Boolean = true,
  contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp),
  responsiveMaxWidth: Dp = ResponsiveContainerDefaults.MaxWidth,
  extraPadding: PaddingValues = PaddingValues(0.dp),
  verticalArrangement: Arrangement.Vertical = Arrangement.Top,
  horizontalAlignment: Alignment.Horizontal = Alignment.Start,
  imeAware: Boolean = false,
  bottomBar: (@Composable BoxScope.() -> Unit)? = null,
  body: (@Composable (contentPadding: PaddingValues) -> Unit)? = null,
  content: @Composable ColumnScope.() -> Unit = {},
) {
  val resolvedScrollState = if (body == null) {
    scrollState ?: rememberScrollState()
  } else {
    scrollState
  }
  val resolvedPrimaryScrollableState = primaryScrollableState ?: resolvedScrollState

  BaseScreen(
    modifier = modifier,
    loading = loading,
    background = background,
    contentPadding = contentPadding,
    responsive = responsive,
    responsiveMaxWidth = responsiveMaxWidth,
    primaryScrollableState = resolvedPrimaryScrollableState,
  ) { adjustedContentPadding ->
    var bottomBarHeight by remember { mutableIntStateOf(0) }
    val density = LocalDensity.current
    val bottomBarPadding = PaddingValues(
      bottom = with(density) { bottomBarHeight.toDp() },
    )
    val resolvedContentPadding = adjustedContentPadding + extraPadding + bottomBarPadding
    val bottomInset = when {
      imeAware -> maxOf(
        WindowInsets.ime.asPaddingValues().calculateBottomPadding(),
        WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding(),
      )
      bottomBar == null -> WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
      else -> 0.dp
    }

    Box(
      modifier = Modifier
        .fillMaxSize()
        .then(if (imeAware && body != null) Modifier.imeOrNavigationBarsPadding() else Modifier),
    ) {
      if (body != null) {
        body(resolvedContentPadding)
      } else {
        ScrollableScreenColumn(
          scrollState = resolvedScrollState ?: error("Screen requires a scroll state when body is not provided"),
          contentPadding = resolvedContentPadding + PaddingValues(bottom = bottomInset),
          verticalArrangement = verticalArrangement,
          horizontalAlignment = horizontalAlignment,
        ) {
          content()
        }
      }

      if (bottomBar != null) {
        Box(
          modifier = Modifier
            .align(Alignment.BottomCenter)
            .fillMaxWidth()
            .then(
              when {
                imeAware && body == null -> Modifier.imeOrNavigationBarsPadding()
                !imeAware -> Modifier.navigationBarsPadding()
                else -> Modifier
              }
            )
            .onSizeChanged { bottomBarHeight = it.height },
        ) {
          bottomBar()
        }
      }
    }
  }
}

@Composable
fun ScrollableScreenColumn(
  scrollState: ScrollState,
  contentPadding: PaddingValues,
  modifier: Modifier = Modifier,
  verticalArrangement: Arrangement.Vertical = Arrangement.Top,
  horizontalAlignment: Alignment.Horizontal = Alignment.Start,
  content: @Composable ColumnScope.() -> Unit,
) {
  Column(
    modifier = Modifier
      .fillMaxSize()
      .verticalScroll(scrollState)
      .padding(contentPadding)
      .then(modifier),
    verticalArrangement = verticalArrangement,
    horizontalAlignment = horizontalAlignment,
  ) {
    content()
  }
}
