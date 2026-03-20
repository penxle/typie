package co.typie.dev

import androidx.compose.runtime.Composable

@Composable
expect fun SystemChrome(content: @Composable () -> Unit)
