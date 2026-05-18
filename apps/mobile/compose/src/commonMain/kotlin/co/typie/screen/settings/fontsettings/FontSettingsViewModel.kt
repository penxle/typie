package co.typie.screen.settings.fontsettings

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.blob.BlobService
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
import co.typie.graphql.type.FontFamilySource
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import co.typie.graphql.type.PersistBlobAsFontInput
import co.typie.graphql.watchQuery
import co.typie.platform.PickedFile
import co.typie.result.Result
import co.typie.result.Task
import co.typie.result.result
import co.typie.result.task
import kotlinx.coroutines.CancellationException

class FontSettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      FontSettingsScreen_Query()
    }

  var isUploading by mutableStateOf(false)
    private set

  val userFontFamilies by derivedStateOf {
    query.data.me.documentFontFamilies
      .filter { it.source == FontFamilySource.USER && it.state == FontFamilyState.ACTIVE }
      .map { family ->
        family.copy(fonts = family.fonts.filter { font -> font.state == FontState.ACTIVE })
      }
  }

  internal fun uploadFonts(
    files: List<PickedFile>
  ): Task<FontUploadProgress, FontUploadResult?, Nothing> = task {
    if (isUploading || files.isEmpty()) return@task null

    isUploading = true

    val successes = mutableListOf<FontUploadSuccess>()
    val failures = mutableListOf<FontUploadFailure>()

    try {
      files.forEachIndexed { index, file ->
        emit(FontUploadProgress(current = index + 1, total = files.size))

        try {
          val path =
            BlobService.uploadBytes(
              bytes = file.bytes,
              filename = file.filename,
              mimeType = file.mimeType,
            )

          val response =
            Apollo.executeMutation(
              FontSettingsScreen_PersistBlobAsFont_Mutation(
                input = PersistBlobAsFontInput(path = path)
              )
            )

          successes +=
            FontUploadSuccess(
              familyId = response.persistBlobAsFont.family.id,
              familyDisplayName = response.persistBlobAsFont.family.displayName,
              weight = response.persistBlobAsFont.weight,
              subfamilyDisplayName = response.persistBlobAsFont.subfamilyDisplayName,
            )
        } catch (e: TypieError) {
          val error =
            when (e.code) {
              "invalid_font_style" -> FontUploadError.InvalidFontStyle
              else -> FontUploadError.Generic
            }
          failures += FontUploadFailure(name = file.filename, error = error)
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          failures += FontUploadFailure(name = file.filename, error = FontUploadError.Generic)
        }
      }
    } finally {
      isUploading = false
    }

    val status =
      when {
        successes.isEmpty() && failures.isNotEmpty() -> FontUploadStatus.Failure
        successes.isNotEmpty() && failures.isEmpty() -> FontUploadStatus.Success
        else -> FontUploadStatus.PartialSuccess
      }

    FontUploadResult(status = status, successes = successes, failures = failures)
  }

  internal suspend fun deleteFamily(familyId: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FontSettingsScreen_ArchiveFontFamily_Mutation(
        input = ArchiveFontFamilyInput(fontFamilyId = familyId)
      )
    )
  }

  internal suspend fun deleteFont(fontId: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      FontSettingsScreen_ArchiveFont_Mutation(input = ArchiveFontInput(fontId = fontId))
    )
  }
}

private fun placeholderData() =
  FontSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { documentFontFamilies = emptyList() }
  }
