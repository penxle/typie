package co.typie.screen.settings.osslicenses

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import org.jetbrains.compose.resources.ExperimentalResourceApi

@OptIn(ExperimentalResourceApi::class)
@Composable
fun OssLicensesScreen() {
  val model = viewModel { OssLicensesViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val sheet = LocalSheet.current

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("오픈소스 라이센스", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        "오픈소스 라이센스",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      SectionTitle("패키지")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        if (model.data.isEmpty()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
          ) {
            Text("표시할 라이센스가 없어요.", style = AppTheme.typography.label)
            Text(
              "의존성 메타데이터가 비어 있어요.",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
            )
          }
        } else {
          Column {
            model.data.separated(separator = { CardDivider() }) { entry ->
              CardRow(
                onClick = { scope.launch { sheet.present { OssLicenseDetailSheet(entry) } } }
              ) {
                OssLicenseRow(entry)
              }
            }
          }
        }
      }
    }
  }
}

@Composable
context(rowScope: RowScope)
private fun OssLicenseRow(entry: OssLicenseEntry) {
  Text(
    text = entry.packageName,
    style = AppTheme.typography.label,
    modifier = with(rowScope) { Modifier.weight(1f) },
    maxLines = 2,
    overflow = TextOverflow.Ellipsis,
  )

  Icon(
    icon = Lucide.ChevronRight,
    modifier = Modifier.size(16.dp),
    tint = AppTheme.colors.textMuted,
  )
}

@Composable
context(_: SheetScope<Unit>)
private fun OssLicenseDetailSheet(entry: OssLicenseEntry) {
  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = entry.packageName,
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    for (paragraph in entry.paragraphs) {
      Text(text = paragraph, style = AppTheme.typography.body, color = AppTheme.colors.textMuted)
    }
  }
}
