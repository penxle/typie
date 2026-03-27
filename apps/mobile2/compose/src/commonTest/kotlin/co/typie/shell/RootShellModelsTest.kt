package co.typie.shell

import co.typie.auth.AuthState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class RootShellModelsTest {
  @Test
  fun `rootShellTargetState differentiates authenticated sessions by session token`() {
    assertNotEquals(
      rootShellTargetState(AuthState.Authenticated, "session-a"),
      rootShellTargetState(AuthState.Authenticated, "session-b"),
    )
  }

  @Test
  fun `rootShellTargetState ignores session token outside authenticated state`() {
    assertEquals(
      rootShellTargetState(AuthState.Unauthenticated, null),
      rootShellTargetState(AuthState.Unauthenticated, "stale-session"),
    )
  }
}
