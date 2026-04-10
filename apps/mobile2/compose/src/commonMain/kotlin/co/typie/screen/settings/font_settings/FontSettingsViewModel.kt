package co.typie.screen.settings.font_settings

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.blob.BlobService
import co.typie.graphql.Apollo
import co.typie.graphql.FontSettingsScreen_ArchiveFontFamily_Mutation
import co.typie.graphql.FontSettingsScreen_ArchiveFont_Mutation
import co.typie.graphql.FontSettingsScreen_PersistBlobAsFont_Mutation
import co.typie.graphql.FontSettingsScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.ArchiveFontFamilyInput
import co.typie.graphql.type.ArchiveFontInput
import co.typie.graphql.type.PersistBlobAsFontInput
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformFile
import co.typie.result.Result
import co.typie.result.Task
import co.typie.result.result
import co.typie.result.task
import kotlinx.coroutines.CancellationException

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

class FontSettingsViewModel : ViewModel() {
  private val blobService = BlobService
  internal val state = FontSettingsScreenState()

  val query = Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { FontSettingsScreen_Query() }

  internal val userFontFamilies: List<FontSettingsFamily>
    get() = uploadedFontFamilies(query.data.me.documentFontFamilies.map { it.toModel() })

  internal fun uploadFonts(files: List<PlatformFile>): Task<FontUploadProgress, FontUploadSummary?, Nothing> = task {
    if (state.isUploading) return@task null
    if (files.isEmpty()) return@task null

    state.isUploading = true
    state.uploadSummary = null

    val successes = mutableListOf<FontUploadSuccess>()
    val failures = mutableListOf<FontUploadFailure>()

    try {
      files.forEachIndexed { index, file ->
        emit(FontUploadProgress(current = index + 1, total = files.size))

        if (!isSupportedTtfFontFile(file.filename, file.mimeType)) {
          failures += FontUploadFailure(
            name = file.filename,
            error = FontUploadError.UnsupportedFormat,
          )
          return@forEachIndexed
        }

        try {
          val path = blobService.uploadBytes(
            bytes = file.bytes,
            filename = file.filename,
            mimeType = file.mimeType ?: "font/ttf",
          )

          val result = Apollo.executeMutation(
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
          val error = when (e.code) {
            "invalid_font_style" -> FontUploadError.InvalidFontStyle
            else -> FontUploadError.UploadFailed
          }
          failures += FontUploadFailure(
            name = file.filename,
            error = error,
          )
        } catch (e: CancellationException) {
          throw e
        } catch (e: Exception) {
          failures += FontUploadFailure(
            name = file.filename,
            error = FontUploadError.UploadFailed,
          )
        }
      }

      if (successes.isNotEmpty()) {
        try {
          query.refetch()
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          failures += FontUploadFailure(
            name = "",
            error = FontUploadError.RefreshFailed,
          )
        }
      }
    } finally {
      state.isUploading = false
    }

    summarizeFontUploadResults(
      successes = successes,
      failures = failures,
    )
  }

  internal fun dismissUploadSummary() {
    state.uploadSummary = null
  }

  internal suspend fun deleteFamily(family: FontSettingsFamily): Result<Unit, Nothing> {
    if (state.deletingFamilyId != null || state.deletingFontId != null) return Result.Ok(Unit)
    state.deletingFamilyId = family.id
    return result<Unit, Nothing> {
      Apollo.executeMutation(
        FontSettingsScreen_ArchiveFontFamily_Mutation(
          input = ArchiveFontFamilyInput(fontFamilyId = family.id),
        ),
      )
      query.refetch()
    }.also { state.deletingFamilyId = null }
  }

  internal suspend fun deleteFont(font: FontSettingsFont): Result<Unit, Nothing> {
    if (state.deletingFamilyId != null || state.deletingFontId != null) return Result.Ok(Unit)
    state.deletingFontId = font.id
    return result<Unit, Nothing> {
      Apollo.executeMutation(
        FontSettingsScreen_ArchiveFont_Mutation(
          input = ArchiveFontInput(fontId = font.id),
        ),
      )
      query.refetch()
    }.also { state.deletingFontId = null }
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
