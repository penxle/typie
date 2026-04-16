package co.typie.domain.entity

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.Apollo
import co.typie.graphql.EntityItemActions_Query
import co.typie.graphql.QueryState
import co.typie.graphql.watchQuery
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.theme.AppTheme

private val EntityItemActionsStatusPadding = 24.dp

private class EntityItemActionsViewModel : ViewModel() {
  var entityId: String by mutableStateOf("")

  val query =
    Apollo.watchQuery(scope = viewModelScope, skip = { entityId.isBlank() }) {
      EntityItemActions_Query(entityId = entityId)
    }
}

@Composable
internal fun rememberEntityItemActionsState(
  entityId: String
): QueryState<EntityItemActions_Query.Data> {
  val model = viewModel { EntityItemActionsViewModel() }

  LaunchedEffect(entityId) { model.entityId = entityId }

  return model.query.state
}

@Composable
context(_: SheetScope<Unit>)
internal fun EntityItemActionsStatusContent(message: String) {
  SheetLayout(padding = SheetPadding.None) {
    Text(
      text = message,
      modifier = Modifier.fillMaxWidth().padding(horizontal = EntityItemActionsStatusPadding),
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
    )
  }
}
