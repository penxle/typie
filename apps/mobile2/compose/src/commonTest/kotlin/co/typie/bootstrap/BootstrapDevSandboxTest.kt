package co.typie.bootstrap

import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull

class BootstrapDevSandboxTest {
  @Test
  fun `effectiveBootstrapState keeps remote state for remote data scenario`() {
    assertEquals(
      BootstrapState.Ready,
      effectiveBootstrapState(
        remoteState = BootstrapState.Ready,
        scenario = BootstrapDevScenario.RemoteData,
      ),
    )
  }

  @Test
  fun `desktop sandbox can override bootstrap state with blocker scenarios`() {
    val sandbox = BootstrapDevSandbox(Platform.Desktop)

    sandbox.select(BootstrapDevScenario.Maintenance)
    assertIs<BootstrapState.Maintenance>(sandbox.currentState)

    sandbox.select(BootstrapDevScenario.UpdateRequired)
    assertIs<BootstrapState.UpdateRequired>(sandbox.currentState)
  }

  @Test
  fun `non desktop sandbox ignores override selections`() {
    val sandbox = BootstrapDevSandbox(Platform.Android)

    sandbox.select(BootstrapDevScenario.Maintenance)

    assertEquals(BootstrapDevScenario.RemoteData, sandbox.scenario.value)
    assertFalse(sandbox.usesSandbox)
    assertNull(sandbox.currentState)
  }
}
