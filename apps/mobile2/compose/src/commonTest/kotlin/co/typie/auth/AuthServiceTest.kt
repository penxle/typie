package co.typie.auth

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class AuthServiceTest {
  @Test
  fun `parseAuthenticatedUserContextResponse extracts user id and site ids from auth bootstrap response`() {
    assertEquals(
      AuthenticatedUserContext(userId = "user_1", siteIds = listOf("site_1", "site_2")),
      parseAuthenticatedUserContextResponse(
        """
        {"data":{"me":{"id":"user_1","sites":[{"id":"site_1"},{"id":"site_2"}]}}}
        """
          .trimIndent()
      ),
    )
  }

  @Test
  fun `parseAuthenticatedUserContextResponse rejects unauthenticated response`() {
    assertFailsWith<IllegalStateException> {
      parseAuthenticatedUserContextResponse("""{"data":{"me":null}}""")
    }
  }
}
