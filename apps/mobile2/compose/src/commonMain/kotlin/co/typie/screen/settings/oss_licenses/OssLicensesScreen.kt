package co.typie.screen.settings.oss_licenses

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.generated.resources.Res
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import org.jetbrains.compose.resources.ExperimentalResourceApi

private sealed interface OssLicensesScreenState {
  data object Loading : OssLicensesScreenState

  data class Loaded(val entries: List<OssLicenseEntry>) : OssLicensesScreenState

  data object Error : OssLicensesScreenState
}

@Serializable
private data class AboutLibrariesPayload(
  val libraries: List<AboutLibrariesLibrary> = emptyList(),
  val licenses: Map<String, AboutLibrariesLicense> = emptyMap(),
)

@Serializable
private data class AboutLibrariesLibrary(
  val uniqueId: String = "",
  val licenses: List<String> = emptyList(),
)

@Serializable
private data class AboutLibrariesLicense(val name: String? = null, val content: String? = null)

private val ossLicensesJson = Json { ignoreUnknownKeys = true }

@OptIn(ExperimentalResourceApi::class)
@Composable
fun OssLicensesScreen() {
  val model = viewModel { OssLicensesViewModel() }
  val bottomSheetHost = LocalBottomSheetHost.current
  val scrollState = rememberScrollState()
  var reloadToken by remember { mutableIntStateOf(0) }
  var state by remember { mutableStateOf<OssLicensesScreenState>(OssLicensesScreenState.Loading) }

  LaunchedEffect(reloadToken) {
    state = OssLicensesScreenState.Loading
    state =
      runCatching {
          val payload =
            ossLicensesJson.decodeFromString<AboutLibrariesPayload>(
              Res.readBytes("files/aboutlibraries.json").decodeToString()
            )
          OssLicensesScreenState.Loaded(aboutLibrariesPayloadToEntries(payload))
        }
        .getOrElse { OssLicensesScreenState.Error }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("오픈소스 라이센스", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    scrollState = scrollState,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text("오픈소스 라이센스", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

    SectionTitle("패키지")

    when (val current = state) {
      OssLicensesScreenState.Loading -> {
        OssLicensesStatusCard(title = "라이센스 정보를 준비하고 있어요.", description = "잠시만 기다려주세요.")
      }

      OssLicensesScreenState.Error -> {
        OssLicensesStatusCard(
          title = "라이센스 정보를 불러오지 못했어요.",
          description = "다시 시도하려면 탭해주세요.",
          onClick = { reloadToken += 1 },
        )
      }

      is OssLicensesScreenState.Loaded -> {
        if (current.entries.isEmpty()) {
          OssLicensesStatusCard(title = "표시할 라이센스가 없어요.", description = "의존성 메타데이터가 비어 있어요.")
        } else {
          CardSurface(modifier = Modifier.fillMaxWidth()) {
            Column {
              current.entries.forEachIndexed { index, entry ->
                CardRow(onClick = { bottomSheetHost.show { OssLicenseDetailSheet(entry) } }) {
                  OssLicenseRowContent(entry)
                }

                if (index < current.entries.lastIndex) {
                  CardDivider()
                }
              }
            }
          }
        }
      }
    }

    Spacer(Modifier.size(72.dp))
  }
}

private fun aboutLibrariesPayloadToEntries(payload: AboutLibrariesPayload): List<OssLicenseEntry> {
  return normalizeOssLicenseEntries(
    payload.libraries.mapNotNull { library ->
      val packageName = library.uniqueId.trim()
      if (packageName.isEmpty()) {
        return@mapNotNull null
      }

      val paragraphs =
        library.licenses.flatMap { licenseId ->
          payload.licenses[licenseId]?.toParagraphs().orEmpty()
        }

      OssLicenseEntry(packageName = packageName, paragraphs = paragraphs)
    }
  )
}

private fun AboutLibrariesLicense.toParagraphs(): List<String> {
  val source =
    content?.trim().takeIf { !it.isNullOrEmpty() }
      ?: name?.trim().takeIf { !it.isNullOrEmpty() }
      ?: return emptyList()

  return source.split(Regex("""\n\s*\n""")).map { it.trim() }.filter { it.isNotEmpty() }
}

@Composable
private fun OssLicensesStatusCard(
  title: String,
  description: String,
  onClick: (suspend () -> Unit)? = null,
) {
  CardSurface(modifier = Modifier.fillMaxWidth()) {
    if (onClick != null) {
      CardRow(onClick = onClick) {
        Column(
          modifier = Modifier.fillMaxWidth(),
          verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
          Text(title, style = AppTheme.typography.label)
          Text(
            description,
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
          )
        }
      }
    } else {
      Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
      ) {
        Text(title, style = AppTheme.typography.label)
        Text(description, style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
      }
    }
  }
}

@Composable
private fun RowScope.OssLicenseRowContent(entry: OssLicenseEntry) {
  Text(
    text = entry.packageName,
    style = AppTheme.typography.label,
    modifier = Modifier.weight(1f),
    maxLines = 2,
    overflow = TextOverflow.Ellipsis,
  )

  Icon(
    icon = Lucide.ChevronRight,
    modifier = Modifier.size(16.dp),
    tint = AppTheme.colors.textTertiary,
  )
}

@Composable
private fun BottomSheetScope<Unit>.OssLicenseDetailSheet(entry: OssLicenseEntry) {
  BottomSheetScaffold(title = entry.packageName) {
    entry.paragraphs.forEach { paragraph ->
      Text(
        text = paragraph,
        style = AppTheme.typography.body,
        color = AppTheme.colors.textSecondary,
      )
    }

    Spacer(Modifier.size(8.dp))
  }
}
