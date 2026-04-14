package co.typie.screen.settings.fontsettings

import co.typie.domain.subscription.Subscription
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Clock
import kotlin.time.Duration.Companion.days

private fun mockSubscription() =
  Subscription(
    id = "sub-1",
    state = SubscriptionState.ACTIVE,
    startsAt = Clock.System.now(),
    expiresAt = Clock.System.now() + 30.days,
    planId = "plan-1",
    planName = "FULL ACCESS",
    fee = 12900,
    availability = PlanAvailability.IN_APP_PURCHASE,
  )

class FontSettingsModelsTest {
  @Test
  fun `uploadedFontFamilies keeps only active user families with active deduplicated fonts`() {
    val families =
      listOf(
        FontSettingsFamily(
          id = "family-default",
          familyName = "Pretendard",
          displayName = "프리텐다드",
          source = "DEFAULT",
          state = "ACTIVE",
          fonts = listOf(FontSettingsFont(id = "default-400", weight = 400)),
        ),
        FontSettingsFamily(
          id = "family-user",
          familyName = "UserSans",
          displayName = "유저 산스",
          source = "USER",
          state = "ACTIVE",
          fonts =
            listOf(
              FontSettingsFont(id = "archived-300", weight = 300, state = "ARCHIVED"),
              FontSettingsFont(id = "regular-old", weight = 400),
              FontSettingsFont(id = "regular-new", weight = 400),
              FontSettingsFont(id = "bold", weight = 700),
            ),
        ),
        FontSettingsFamily(
          id = "family-archived",
          familyName = "UserSerif",
          displayName = "유저 세리프",
          source = "USER",
          state = "ARCHIVED",
          fonts = listOf(FontSettingsFont(id = "archived", weight = 400)),
        ),
      )

    val result = uploadedFontFamilies(families)

    assertEquals(1, result.size)
    assertEquals(listOf("regular-new", "bold"), result.single().fonts.map { it.id })
  }

  @Test
  fun `representativeFont prefers weight closest to 400 and breaks ties toward heavier`() {
    val result =
      representativeFont(
        listOf(
          FontSettingsFont(id = "light", weight = 300),
          FontSettingsFont(id = "medium", weight = 500),
          FontSettingsFont(id = "bold", weight = 700),
        )
      )

    assertEquals("medium", result?.id)
  }

  @Test
  fun `representativeFont returns null for empty input`() {
    assertNull(representativeFont(emptyList()))
  }

  @Test
  fun `fontWeightLabel uses shared weight labels before subfamily fallback`() {
    assertEquals("보통", fontWeightLabel(weight = 400, subfamilyDisplayName = "Regular"))
    assertEquals(
      "Semi Condensed (450)",
      fontWeightLabel(weight = 450, subfamilyDisplayName = "Semi Condensed"),
    )
    assertEquals("950", fontWeightLabel(weight = 950, subfamilyDisplayName = null))
  }

  @Test
  fun `isSupportedTtfFontFile accepts ttf extension and known ttf mime types only`() {
    assertTrue(isSupportedTtfFontFile(filename = "MyFont.ttf", mimeType = null))
    assertTrue(isSupportedTtfFontFile(filename = "font", mimeType = "font/ttf"))
    assertTrue(isSupportedTtfFontFile(filename = "font", mimeType = "application/x-font-ttf"))
    assertFalse(isSupportedTtfFontFile(filename = "MyFont.otf", mimeType = "font/otf"))
  }

  @Test
  fun `fontUploadAction returns plan upgrade sheet for unsubscribed users`() {
    assertEquals(
      FontUploadAction.PickFont,
      fontUploadAction(SubscriptionServiceState.Subscribed(subscription = mockSubscription())),
    )
    assertEquals(
      FontUploadAction.ShowPlanUpgradeSheet,
      fontUploadAction(SubscriptionServiceState.NotSubscribed),
    )
    assertNull(fontUploadAction(SubscriptionServiceState.Unknown))
  }

  @Test
  fun `summarizeFontUploadResults groups successful uploads by family`() {
    val summary =
      summarizeFontUploadResults(
        successes =
          listOf(
            FontUploadSuccess(
              familyId = "family-1",
              familyDisplayName = "프리텐다드",
              weight = 400,
              subfamilyDisplayName = "Regular",
            ),
            FontUploadSuccess(
              familyId = "family-1",
              familyDisplayName = "프리텐다드",
              weight = 700,
              subfamilyDisplayName = "Bold",
            ),
            FontUploadSuccess(
              familyId = "family-2",
              familyDisplayName = "코펍월드돋움",
              weight = 450,
              subfamilyDisplayName = "Semi Condensed",
            ),
          ),
        failures = emptyList(),
      )

    assertNotNull(summary)
    assertEquals(FontUploadSummaryStatus.Success, summary.status)
    assertEquals(3, summary.successes.size)
    assertEquals(0, summary.failures.size)
  }

  @Test
  fun `summarizeFontUploadResults includes both success and failure for partial uploads`() {
    val summary =
      summarizeFontUploadResults(
        successes =
          listOf(
            FontUploadSuccess(
              familyId = "family-1",
              familyDisplayName = "프리텐다드",
              weight = 400,
              subfamilyDisplayName = "Regular",
            )
          ),
        failures =
          listOf(FontUploadFailure(name = "Italic.ttf", error = FontUploadError.InvalidFontStyle)),
      )

    assertNotNull(summary)
    assertEquals(FontUploadSummaryStatus.PartialSuccess, summary.status)
    assertEquals(1, summary.successes.size)
    assertEquals(1, summary.failures.size)
    assertEquals("Italic.ttf", summary.failures.first().name)
    assertEquals(FontUploadError.InvalidFontStyle, summary.failures.first().error)
  }

  @Test
  fun `summarizeFontUploadResults returns failure summary when all uploads fail`() {
    val summary =
      summarizeFontUploadResults(
        successes = emptyList(),
        failures =
          listOf(
            FontUploadFailure(name = "Broken.otf", error = FontUploadError.UnsupportedFormat),
            FontUploadFailure(name = "Italic.ttf", error = FontUploadError.InvalidFontStyle),
          ),
      )

    assertNotNull(summary)
    assertEquals(FontUploadSummaryStatus.Failure, summary.status)
    assertEquals(0, summary.successes.size)
    assertEquals(2, summary.failures.size)
    assertEquals("Broken.otf", summary.failures[0].name)
    assertEquals("Italic.ttf", summary.failures[1].name)
  }

  @Test
  fun `summarizeFontUploadResults returns null when there is nothing to report`() {
    assertNull(summarizeFontUploadResults(successes = emptyList(), failures = emptyList()))
  }
}
