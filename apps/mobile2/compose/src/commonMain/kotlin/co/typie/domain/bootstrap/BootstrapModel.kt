package co.typie.domain.bootstrap

sealed interface BootstrapState {
  data object NotReady : BootstrapState

  data object Ready : BootstrapState
}
