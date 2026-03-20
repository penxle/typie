package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.ui.theme.AppTheme

@Composable
fun Screen(
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  Box(
    modifier
      .fillMaxSize()
      .background(AppTheme.colors.surfaceDefault),
  ) {
    content()
  }
}
