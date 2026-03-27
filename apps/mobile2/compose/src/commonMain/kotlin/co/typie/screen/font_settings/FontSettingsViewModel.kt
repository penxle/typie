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

class FontSettingsScreenState {
  var isUploading by mutableStateOf(false)
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
  val state = FontSettingsScreenState()

  val query = watchQuery(placeholderData()) { FontSettingsScreen_Query() }

  internal val hasSubscription: Boolean
    get() = query.data.me.subscription != null

  internal val userFontFamilies: List<FontSettingsFamily>
    get() = uploadedFontFamilies(query.data.me.documentFontFamilies.map { it.toModel() })

  internal suspend fun uploadFont(file: PlatformFile) {
    if (state.isUploading) return

    if (!isSupportedTtfFontFile(file.filename, file.mimeType)) {
      toast.show(type = co.typie.overlay.ToastType.Error, message = "TTF 파일만 업로드할 수 있어요.")
      return
    }

    state.isUploading = true
    try {
      toast.withLoading(
        message = "폰트 업로드 중...",
        errorMessage = "폰트 업로드에 실패했어요. 다시 시도해주세요.",
      ) {
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

        query.refetch()

        val uploadedLabel = fontWeightLabel(
          weight = result.persistBlobAsFont.weight,
          subfamilyDisplayName = result.persistBlobAsFont.subfamilyDisplayName,
        )

        success("${result.persistBlobAsFont.family.displayName} $uploadedLabel 폰트가 업로드되었어요.")
      }
    } catch (e: TypieError) {
      when (e.code) {
        "invalid_font_style" -> toast.show(
          type = co.typie.overlay.ToastType.Error,
          message = "기울어진 폰트는 업로드할 수 없어요.",
        )

        else -> {
          Logger.e(e) { "Failed to upload font: ${e.code}" }
        }
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to upload font" }
    } finally {
      state.isUploading = false
    }
  }

  internal fun showUploadSubscriptionNotice() {
    // TODO: 구독 화면이 준비되면 업그레이드 화면으로 연결
    toast.show(
      type = ToastType.Notification,
      message = "폰트 업로드는 FULL ACCESS 플랜에서 사용할 수 있어요.",
    )
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
