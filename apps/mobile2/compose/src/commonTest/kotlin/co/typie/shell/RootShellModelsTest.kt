package co.typie.shell

import co.typie.auth.AuthState
import co.typie.bootstrap.BootstrapState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class RootShellModelsTest {
  @Test
  fun `rootShellTargetState differentiates authenticated sessions by session token`() {
    assertNotEquals(
      rootShellTargetState(AuthState.Authenticated, "session-a", BootstrapState.Ready),
      rootShellTargetState(AuthState.Authenticated, "session-b", BootstrapState.Ready),
    )
  }

  @Test
  fun `rootShellTargetState ignores session token outside authenticated state`() {
    assertEquals(
      rootShellTargetState(AuthState.Unauthenticated, null, BootstrapState.Ready),
      rootShellTargetState(AuthState.Unauthenticated, "stale-session", BootstrapState.Ready),
    )
  }

  @Test
  fun `resolveRootShellDestination prioritizes bootstrap blockers before auth destinations`() {
    assertEquals(
      RootShellDestination.Maintenance(
        title = "점검 중",
        message = "잠시 후 다시 시도해주세요.",
        until = null,
      ),
      resolveRootShellDestination(
        authState = AuthState.Authenticated,
        bootstrapState = BootstrapState.Maintenance(
          title = "점검 중",
          message = "잠시 후 다시 시도해주세요.",
          until = null,
        ),
      ),
    )
    assertEquals(
      RootShellDestination.Offline,
      resolveRootShellDestination(
        authState = AuthState.Offline,
        bootstrapState = BootstrapState.Ready,
      ),
    )
  }
}
