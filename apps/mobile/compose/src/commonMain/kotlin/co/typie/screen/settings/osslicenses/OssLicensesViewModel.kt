package co.typie.screen.settings.osslicenses

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.contract.Loadable
import co.typie.contract.LoadableState
import co.typie.generated.resources.Res
import co.typie.serialization.json
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch

internal class OssLicensesViewModel : ViewModel(), Loadable<List<OssLicenseEntry>> {
  override var state by mutableStateOf<LoadableState<List<OssLicenseEntry>>>(LoadableState.Idle)
    private set

  val data by derivedStateOf { (state as? LoadableState.Success)?.data ?: emptyList() }

  override fun refetch() {
    viewModelScope.launch {
      try {
        val aboutLibraries =
          json.decodeFromString<AboutLibraries>(
            Res.readBytes("files/aboutlibraries.json").decodeToString()
          )
        state = LoadableState.Success(aboutLibraries.toEntry())
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        state = LoadableState.Error(e)
      }
    }
  }
}
