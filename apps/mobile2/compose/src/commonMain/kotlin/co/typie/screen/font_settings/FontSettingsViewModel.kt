package co.typie.screen.font_settings

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.blob.BlobService
import co.typie.graphql.FontSettingsScreen_ArchiveFont_Mutation
import co.typie.graphql.FontSettingsScreen_ArchiveFontFamily_Mutation
import co.typie.graphql.FontSettingsScreen_PersistBlobAsFont_Mutation
import co.typie.graphql.FontSettingsScreen_Query
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.type.ArchiveFontFamilyInput
import co.typie.graphql.type.ArchiveFontInput
import co.typie.graphql.type.PersistBlobAsFontInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.PlatformFile
import kotlinx.coroutines.CancellationException
import org.koin.core.annotation.KoinViewModel
import kotlin.time.Duration

internal class FontSettingsScreenState {
  var isUploading by mutableStateOf(false)
    internal set

  var uploadCurrentIndex by mutableStateOf(0)
    internal set

  var uploadTotalCount by mutableStateOf(0)
    internal set

  var uploadSummary: FontUploadSummary? by mutableStateOf(null)
    internal set

  var deletingFamilyId: String? by mutableStateOf(null)
    internal set

  var deletingFontId: String? by mutableStateOf(null)
    internal set
}

@KoinViewModel
class FontSettingsViewModel(
  private val blobService: BlobService,
  private val toast: Toast,
) : GraphQLViewModel() {
  internal val state = FontSettingsScreenState()

  val query = watchQuery(placeholderData()) { FontSettingsScreen_Query() }

  internal val userFontFamilies: List<FontSettingsFamily>
    get() = uploadedFontFamilies(query.data.me.documentFontFamilies.map { it.toModel() })

  internal suspend fun uploadFonts(files: List<PlatformFile>) {
    if (state.isUploading) return
    if (files.isEmpty()) return

    state.isUploading = true
    state.uploadSummary = null
    state.uploadCurrentIndex = 0
    state.uploadTotalCount = files.size

    val successes = mutableListOf<FontUploadSuccess>()
    val failures = mutableListOf<FontUploadFailure>()

    try {
      files.forEachIndexed { index, file ->
        state.uploadCurrentIndex = index + 1
        toast.show(
          type = ToastType.Loading,
          message = "폰트 업로드 중... (${state.uploadCurrentIndex}/${state.uploadTotalCount})",
          duration = Duration.ZERO,
        )

        if (!isSupportedTtfFontFile(file.filename, file.mimeType)) {
          failures += FontUploadFailure(
            name = file.filename,
            error = "TTF 파일만 업로드할 수 있어요.",
          )
          return@forEachIndexed
        }

        try {
          val path = blobService.uploadBytes(
            bytes = file.bytes,
            filename = file.filename,
            mimeType = file.mimeType ?: "font/ttf",
          )

          val result = executeMutation(
            FontSettingsScreen_PersistBlobAsFont_Mutation(
              input = PersistBlobAsFontInput(path = path),
            ),
          )

          successes += FontUploadSuccess(
            familyId = result.persistBlobAsFont.family.id,
            familyDisplayName = result.persistBlobAsFont.family.displayName,
            weight = result.persistBlobAsFont.weight,
            subfamilyDisplayName = result.persistBlobAsFont.subfamilyDisplayName,
          )
        } catch (e: TypieError) {
          val message = when (e.code) {
            "invalid_font_style" -> "기울어진 폰트는 업로드할 수 없어요."
            else -> "폰트 업로드에 실패했어요."
          }
          Logger.e(e) { "Failed to upload font: ${e.code}" }
          failures += FontUploadFailure(
            name = file.filename,
            error = message,
          )
        } catch (e: CancellationException) {
          throw e
        } catch (e: Exception) {
          Logger.e(e) { "Failed to upload font" }
          failures += FontUploadFailure(
            name = file.filename,
            error = "폰트 업로드에 실패했어요.",
          )
        }
      }

      if (successes.isNotEmpty()) {
        try {
          query.refetch()
        } catch (e: CancellationException) {
          throw e
        } catch (e: Exception) {
          Logger.e(e) { "Failed to refetch font settings after upload" }
          failures += FontUploadFailure(
            name = "업로드 결과 반영",
            error = "폰트 목록을 새로고침하지 못했어요. 화면을 다시 열어주세요.",
          )
        }
      }
    } catch (e: CancellationException) {
      throw e
    } finally {
      toast.dismiss()
      state.uploadSummary = summarizeFontUploadResults(
        successes = successes,
        failures = failures,
      )
      state.uploadCurrentIndex = 0
      state.uploadTotalCount = 0
      state.isUploading = false
    }
  }

  internal fun dismissUploadSummary() {
    state.uploadSummary = null
  }

  internal suspend fun deleteFamily(family: FontSettingsFamily) {
    if (state.deletingFamilyId != null || state.deletingFontId != null) return

    state.deletingFamilyId = family.id
    try {
      executeMutation(
        FontSettingsScreen_ArchiveFontFamily_Mutation(
          input = ArchiveFontFamilyInput(fontFamilyId = family.id),
        ),
      )

      query.refetch()
      toast.show(type = co.typie.overlay.ToastType.Success, message = "\"${family.displayName}\" 폰트 패밀리를 삭제했어요.")
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to delete font family" }
      toast.show(type = co.typie.overlay.ToastType.Error, message = "폰트 패밀리 삭제에 실패했어요.")
    } finally {
      state.deletingFamilyId = null
    }
  }

  internal suspend fun deleteFont(
    familyDisplayName: String,
    font: FontSettingsFont,
  ) {
    if (state.deletingFamilyId != null || state.deletingFontId != null) return

    state.deletingFontId = font.id
    try {
      executeMutation(
        FontSettingsScreen_ArchiveFont_Mutation(
          input = ArchiveFontInput(fontId = font.id),
        ),
      )

      query.refetch()
      toast.show(
        type = co.typie.overlay.ToastType.Success,
        message = "\"$familyDisplayName ${fontWeightLabel(font.weight, font.subfamilyDisplayName)}\" 폰트를 삭제했어요.",
      )
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to delete font" }
      toast.show(type = co.typie.overlay.ToastType.Error, message = "폰트 삭제에 실패했어요.")
    } finally {
      state.deletingFontId = null
    }
  }
}

private fun FontSettingsScreen_Query.DocumentFontFamily.toModel(): FontSettingsFamily {
  return FontSettingsFamily(
    id = id,
    familyName = familyName,
    displayName = displayName,
    source = source.rawValue,
    state = state.rawValue,
    fonts = fonts.map { it.toModel() },
  )
}

private fun FontSettingsScreen_Query.Font.toModel(): FontSettingsFont {
  return FontSettingsFont(
    id = id,
    weight = weight,
    subfamilyDisplayName = subfamilyDisplayName,
    state = state.rawValue,
  )
}

private fun placeholderData() = FontSettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    subscription = null
    documentFontFamilies = emptyList()
  }
}
