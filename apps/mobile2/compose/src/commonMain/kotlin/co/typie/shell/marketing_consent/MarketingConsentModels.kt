package co.typie.shell.marketing_consent

import kotlin.time.Instant

internal const val MARKETING_CONSENT_CHARACTER_THRESHOLD = 100

internal fun shouldShowMarketingConsentPrompt(
  marketingConsentAskedAt: Instant?,
  totalCharacterCount: Int,
): Boolean {
  return marketingConsentAskedAt == null && totalCharacterCount >= MARKETING_CONSENT_CHARACTER_THRESHOLD
}
