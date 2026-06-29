package co.typie.graphql

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.contract.LoadableState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

class LoadableMutation<D> {
  var state: LoadableState<D> by mutableStateOf(LoadableState.Idle)
    private set

  private var job: Job? = null
  private var token = 0L

  val loading: Boolean
    get() = state is LoadableState.Loading

  val data: D?
    get() = (state as? LoadableState.Success<D>)?.data

  val error: Throwable?
    get() = (state as? LoadableState.Error)?.exception

  fun run(
    scope: CoroutineScope,
    replaceInFlight: Boolean = false,
    block: suspend () -> D,
    onSuccess: (D) -> Unit = {},
    onError: (Throwable) -> Unit = {},
  ) {
    if (loading && !replaceInFlight) return
    if (replaceInFlight) job?.cancel()

    val runToken = ++token
    state = LoadableState.Loading
    val nextJob =
      scope.launch(start = CoroutineStart.LAZY) {
        try {
          val result =
            try {
              Result.success(block())
            } catch (error: Throwable) {
              if (error is CancellationException) throw error
              Result.failure(error)
            }
          if (runToken != token) return@launch
          result.fold(
            onSuccess = { data ->
              state = LoadableState.Success(data)
              onSuccess(data)
            },
            onFailure = { error ->
              state = LoadableState.Error(error)
              onError(error)
            },
          )
        } finally {
          if (runToken == token && job === coroutineContext[Job]) {
            job = null
          }
        }
      }
    job = nextJob
    nextJob.start()
  }

  fun reset() {
    token++
    job?.cancel()
    job = null
    state = LoadableState.Idle
  }

  fun cancel() {
    reset()
  }
}
