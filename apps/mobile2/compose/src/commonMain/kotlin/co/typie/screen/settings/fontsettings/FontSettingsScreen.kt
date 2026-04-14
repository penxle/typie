package co.typie.screen.settings.fontsettings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.graphql.FontSettingsScreen_Query
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.FilePickerSelectionMode
import co.typie.platform.rememberFilePicker
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.FontSpecimen
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.time.Duration
import kotlinx.coroutines.launch

@Composable
fun FontSettingsScreen() {
  val model = viewModel { FontSettingsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val sheet = LocalSheet.current

  val filePicker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { files ->
      if (files.isEmpty()) return@rememberFilePicker

      scope.launch {
        model
          .uploadFonts(files)
          .collect(
            onPending = { progress ->
              toast.show(
                ToastType.Loading,
                "폰트 업로드 중... (${progress.current}/${progress.total})",
                Duration.ZERO,
              )
            },
            onSettled = { result ->
              toast.dismiss()
              val settled = result.withDefaultExceptionHandler(toast)
              if (settled is Result.Ok && settled.value != null) {
                val (title, message) = fontUploadSummaryDisplay(settled.value)
                dialog.alert(title = title, message = message)
              }
            },
          )
      }
    }

  suspend fun uploadFonts() {
    if (model.isUploading) return

    val passed =
      SubscriptionService.gate(sheet, nav, message = "폰트 업로드 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.")

    if (passed) {
      sheet.present {
        FontUploadSheet(isUploading = model.isUploading, onUpload = { filePicker("*/*") })
      }
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("폰트", style = AppTheme.typography.title) },
    trailing = { TopBarButton(Lucide.Plus, onClick = { uploadFonts() }) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(query = model.query) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text("폰트", style = AppTheme.typography.display)

      SectionTitle("직접 업로드한 폰트")

      if (model.userFontFamilies.isEmpty()) {
        CardSurface(modifier = Modifier.fillMaxWidth()) {
          Text(
            "아직 직접 업로드한 폰트가 없어요.\n우측 상단의 추가 버튼으로 TTF 폰트를 한 번에 여러 개 업로드할 수 있어요.",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 40.dp),
          )
        }
      } else {
        for (family in model.userFontFamilies) {
          FontSettingsFamilySection(
            family = family,
            onDeleteFontFamily = {
              val result =
                dialog.confirm(
                  title = "폰트 패밀리 삭제",
                  message = """"${family.displayName}" 폰트 패밀리 전체를 삭제하시겠어요?""",
                  confirmText = "삭제",
                  confirmIsDestructive = true,
                )

              if (result is DialogResult.Resolved) {
                model.deleteFamily(family.id).withDefaultExceptionHandler(toast).onOk {
                  toast.success(""""${family.displayName}" 폰트 패밀리를 삭제했어요.""")
                }
              }
            },
            onDeleteFont = { font ->
              val result =
                dialog.confirm(
                  title = "폰트 삭제",
                  message =
                    """"${family.displayName} ${fontWeightLabel(font.weight, font.subfamilyDisplayName)}" 폰트를 삭제하시겠어요?""",
                  confirmText = "삭제",
                  confirmIsDestructive = true,
                )

              if (result is DialogResult.Resolved) {
                model.deleteFont(font.id).withDefaultExceptionHandler(toast).onOk {
                  toast.success(
                    """"${family.displayName} ${fontWeightLabel(font.weight, font.subfamilyDisplayName)}" 폰트를 삭제했어요."""
                  )
                }
              }
            },
          )
        }
      }
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun FontUploadSheet(isUploading: Boolean, onUpload: suspend () -> Unit) {
  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "폰트 업로드하기",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
    footer = {
      Button(
        text = "폰트 파일 선택",
        loading = isUploading,
        loadingText = "업로드 중...",
        onClick = {
          dismiss()
          onUpload()
        },
      )
    },
  ) {
    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Text("이용 안내", style = AppTheme.typography.label)

        BulletItem("TTF 확장자를 가진 폰트 파일만 업로드할 수 있어요.")
        BulletItem("여러 개의 TTF 폰트 파일을 한 번에 선택할 수 있어요.")
        BulletItem("기울어진 폰트는 업로드할 수 없어요.")
        BulletItem("업로드한 폰트는 내 글이라면 어디서나 사용할 수 있어요.")
        BulletItem("무료 폰트이거나 웹 사용 라이선스가 있는 폰트만 이용해 주세요.")
        BulletItem("저작권에 위배되는 폰트는 삭제될 수 있어요.")
      }
    }
  }
}

@Composable
private fun BulletItem(text: String) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
    verticalAlignment = Alignment.Top,
  ) {
    Text("•", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
    Text(
      text,
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.weight(1f),
    )
  }
}

@Composable
private fun FontSettingsFamilySection(
  family: FontSettingsScreen_Query.DocumentFontFamily,
  onDeleteFontFamily: suspend () -> Unit,
  onDeleteFont: suspend (font: FontSettingsScreen_Query.Font) -> Unit,
) {
  val representativeFont = family.representativeFont

  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    FontSpecimen(
      fontId = representativeFont.id,
      text = family.displayName,
      fallbackTexts = listOf(family.familyName),
      style = AppTheme.typography.title.copy(fontWeight = FontWeight(representativeFont.weight)),
      modifier = Modifier.fillMaxWidth(),
    )

    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column {
        family.fonts.separated(separator = { CardDivider() }) { font ->
          val weightLabel = fontWeightLabel(font.weight, font.subfamilyDisplayName)

          Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            FontSpecimen(
              fontId = font.id,
              text = weightLabel,
              fallbackTexts = listOfNotNull(font.subfamilyDisplayName, font.weight.toString()),
              style = AppTheme.typography.label.copy(fontWeight = FontWeight(font.weight)),
              modifier = Modifier.weight(1f),
            )

            InteractionScope {
              Box(
                modifier = Modifier.size(44.dp).clickable { onDeleteFont(font) }.pressScale(),
                contentAlignment = Alignment.Center,
              ) {
                Icon(
                  icon = Lucide.Trash2,
                  modifier = Modifier.size(18.dp),
                  tint = AppTheme.colors.textSecondary,
                )
              }
            }
          }
        }

        if (family.fonts.size > 1) {
          CardDivider()

          InteractionScope {
            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .clickable { onDeleteFontFamily() }
                  .padding(horizontal = 16.dp, vertical = 14.dp)
                  .pressScale(),
              contentAlignment = Alignment.Center,
            ) {
              Text(
                "이 폰트 패밀리 전체 삭제",
                style = AppTheme.typography.action,
                color = AppTheme.colors.danger,
              )
            }
          }
        }
      }
    }
  }
}

private fun fontUploadSummaryDisplay(summary: FontUploadResult): Pair<String, String> {
  val title =
    when (summary.status) {
      FontUploadStatus.Success -> "폰트 업로드 완료"
      FontUploadStatus.PartialSuccess -> "폰트 업로드 일부 완료"
      FontUploadStatus.Failure -> "폰트 업로드 실패"
    }

  val sections = buildList {
    if (summary.successes.isNotEmpty()) {
      val successesByFamily = linkedMapOf<String, MutableList<FontUploadSuccess>>()
      summary.successes.forEach { success ->
        successesByFamily.getOrPut(success.familyId) { mutableListOf() }.add(success)
      }

      val successLines =
        successesByFamily.values.map { familyUploads ->
          val familyDisplayName = familyUploads.first().familyDisplayName
          val labels =
            familyUploads
              .sortedBy { it.weight }
              .joinToString(", ") { fontWeightLabel(it.weight, it.subfamilyDisplayName) }
          "\u2022 $familyDisplayName ($labels)"
        }

      add("${summary.successes.size}개의 폰트가 추가되었어요.\n\n${successLines.joinToString("\n")}")
    }

    if (summary.failures.isNotEmpty()) {
      val failureLines =
        summary.failures.joinToString("\n") { failure ->
          val errorMessage =
            when (failure.error) {
              FontUploadError.InvalidFontStyle -> "기울어진 폰트는 업로드할 수 없어요."
              FontUploadError.Generic -> "폰트 업로드에 실패했어요."
            }
          if (failure.name.isNotEmpty()) "\u2022 ${failure.name}: $errorMessage"
          else "\u2022 $errorMessage"
        }
      add("${summary.failures.size}개의 폰트 업로드에 실패했어요.\n\n$failureLines")
    }
  }

  val note =
    if (summary.status == FontUploadStatus.Success) {
      "업로드한 폰트는 이 화면에서 관리할 수 있어요."
    } else {
      null
    }

  val message = buildString {
    append(sections.joinToString("\n\n"))
    if (note != null) {
      append("\n\n")
      append(note)
    }
  }

  return title to message
}

private val FontSettingsScreen_Query.DocumentFontFamily.representativeFont
  get() =
    this.fonts.reduce { previous, current ->
      val previousDiff = abs(previous.weight - 400)
      val currentDiff = abs(current.weight - 400)

      when {
        currentDiff < previousDiff -> current
        currentDiff == previousDiff && current.weight > previous.weight -> current
        else -> previous
      }
    }
