package co.typie.graphql

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.navigation.Nav
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun <D> GraphQLContent(
  result: QueryResult<D>,
  modifier: Modifier = Modifier,
  content: @Composable (data: D) -> Unit,
) {
  when (val state = result.state) {
    is QueryState.Loading -> {
      Box(modifier.fillMaxSize())
    }

    is QueryState.Error -> {
      GraphQLErrorContent(
        onRetry = result.refetch,
        modifier = modifier,
      )
    }

    is QueryState.Success -> {
      AnimatedVisibility(
        visible = true,
        enter = fadeIn(tween(200)),
      ) {
        content(state.data)
      }
    }
  }
}

@Composable
private fun GraphQLErrorContent(
  onRetry: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val nav = Nav.current

  Column(
    modifier = modifier.fillMaxSize(),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.Center,
  ) {
    Text("앗! 문제가 발생했어요")
    Text(
      "잠시 후 다시 시도해주세요.",
      style = AppTheme.typography.action,
      color = AppTheme.colors.textFaint,
    )
    Spacer(Modifier.height(16.dp))
    Box(
      modifier = Modifier.border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
        .clickable(onRetry).padding(horizontal = 16.dp, vertical = 8.dp),
    ) {
      Text("다시 시도하기", style = AppTheme.typography.action)
    }
    if (nav.canPop) {
      Spacer(Modifier.height(8.dp))
      Box(
        modifier = Modifier.border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
          .clickable { nav.pop() }.padding(horizontal = 16.dp, vertical = 8.dp),
      ) {
        Text("뒤로 가기", style = AppTheme.typography.action)
      }
    }
  }
}
