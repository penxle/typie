package co.typie.ui.component

import androidx.compose.ui.graphics.Color
import co.typie.ui.utils.toHexRgbString
import kotlin.test.Test
import kotlin.test.assertEquals

class FontSpecimenTest {
  @Test
  fun `fontSpecimenUrl includes encoded fallback and color query params`() {
    val result = fontSpecimenUrl(
      fontId = "font-123",
      text = "보통",
      fallbackTexts = listOf("Regular", "400"),
      colorHex = "#F1F1F7",
    )

    assertEquals(
      "${co.typie.Konfig.API_URL}/font/font-123/specimen?text=%EB%B3%B4%ED%86%B5&fallbacks=Regular&fallbacks=400&color=%23F1F1F7",
      result,
    )
  }

  @Test
  fun `fontSpecimenUrl omits blank optional query params`() {
    val result = fontSpecimenUrl(
      fontId = "font-123",
      text = "Bold",
      fallbackTexts = listOf(" ", "Bold"),
      colorHex = null,
    )

    assertEquals(
      "${co.typie.Konfig.API_URL}/font/font-123/specimen?text=Bold",
      result,
    )
  }

  @Test
  fun `familySpecimenFallbacks returns family name only when it differs from display name`() {
    assertEquals(
      listOf("Pretendard"),
      familySpecimenFallbacks(
        displayName = "프리텐다드",
        familyName = "Pretendard",
      ),
    )

    assertEquals(
      emptyList(),
      familySpecimenFallbacks(
        displayName = "Pretendard",
        familyName = "Pretendard",
      ),
    )
  }

  @Test
  fun `weightSpecimenFallbacks keeps secondary text and numeric fallback`() {
    assertEquals(
      listOf("Regular", "400"),
      weightSpecimenFallbacks(
        label = "보통",
        subfamilyDisplayName = "Regular",
        weight = 400,
      ),
    )

    assertEquals(
      listOf("400"),
      weightSpecimenFallbacks(
        label = "보통",
        subfamilyDisplayName = null,
        weight = 400,
      ),
    )

    assertEquals(
      emptyList(),
      weightSpecimenFallbacks(
        label = "400",
        subfamilyDisplayName = null,
        weight = 400,
      ),
    )
  }

  @Test
  fun `toHexRgbString formats opaque colors as uppercase hex`() {
    assertEquals("#F1F1F7", Color(0xFFF1F1F7).toHexRgbString())
  }
}
