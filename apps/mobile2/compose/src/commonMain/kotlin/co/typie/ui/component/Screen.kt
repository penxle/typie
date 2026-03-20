package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.ext.statusBars
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource

@Composable
fun Screen(
  modifier: Modifier = Modifier,
  topBar: (@Composable () -> Unit)? = null,
  content: @Composable (contentPadding: PaddingValues) -> Unit,
) {
  val colors = AppTheme.colors
  val hazeState = remember { HazeState() }
  val statusBarTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
  val contentPadding = if (topBar != null) {
    PaddingValues(top = statusBarTop + TopBarDefaults.Height + TopBarDefaults.BlurFadeHeight + TopBarDefaults.ContentTopSpacing)
  } else {
    PaddingValues()
  }


  Box(
    modifier
      .fillMaxSize()
      .background(colors.surfaceDefault),
  ) {
    Box(
      Modifier
        .fillMaxSize()
        .then(if (topBar != null) Modifier.hazeSource(hazeState) else Modifier),
    ) {
      content(contentPadding)
    }

    if (topBar != null) {
      Column(
        modifier = Modifier
          .fillMaxWidth()
          .align(Alignment.TopStart)
          .hazeEffect(hazeState) {
            backgroundColor = colors.surfaceDefault
            blurRadius = TopBarDefaults.BlurRadius
            noiseFactor = 0f
            progressive = TopBarDefaults.hazeProgressive()
          },
      ) {
        topBar()
        Spacer(Modifier.height(TopBarDefaults.BlurFadeHeight))
      }
    }
  }
}
