package co.typie.ui.component.bottomsheet

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface

@Composable
fun BottomSheetActionList(
  modifier: Modifier = Modifier,
  content: @Composable ColumnScope.() -> Unit,
) {
  CardSurface(
    modifier = modifier.fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier.fillMaxWidth(),
      content = content,
    )
  }
}

@Composable
fun BottomSheetActionRow(
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 16.dp),
  content: @Composable RowScope.() -> Unit,
) {
  CardRow(
    onClick = { if (enabled) onClick() },
    modifier = modifier,
    contentPadding = contentPadding,
    content = content,
  )
}

@Composable
fun BottomSheetActionDivider(
  modifier: Modifier = Modifier,
) {
  CardDivider(modifier = modifier)
}
