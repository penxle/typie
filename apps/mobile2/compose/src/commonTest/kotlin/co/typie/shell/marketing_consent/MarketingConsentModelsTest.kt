package co.typie.shell.marketing_consent

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Instant

class MarketingConsentModelsTest {
  @Test
  fun `shows prompt when not asked yet and threshold reached`() {
    assertTrue(
      shouldShowMarketingConsentPrompt(
        marketingConsentAskedAt = null,
        totalCharacterCount = MARKETING_CONSENT_CHARACTER_THRESHOLD,
      )
    )
  }

  @Test
  fun `does not show prompt below threshold`() {
    assertFalse(
      shouldShowMarketingConsentPrompt(
        marketingConsentAskedAt = null,
        totalCharacterCount = MARKETING_CONSENT_CHARACTER_THRESHOLD - 1,
      )
    )
  }

  @Test
  fun `does not show prompt once already asked`() {
    assertFalse(
      shouldShowMarketingConsentPrompt(
        marketingConsentAskedAt = Instant.parse("2026-03-27T00:00:00Z"),
        totalCharacterCount = MARKETING_CONSENT_CHARACTER_THRESHOLD + 1,
      )
    )
  }
}
