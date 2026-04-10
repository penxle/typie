package co.typie.auth.sso

import android.app.Activity
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import co.typie.Konfig
import co.typie.graphql.type.SingleSignOnProvider
import com.google.android.libraries.identity.googleid.GetSignInWithGoogleOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential

actual class GoogleSingleSignOnProvider : SingleSignOnAdapter {

  actual override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    val context = ctx as Activity
    val option = GetSignInWithGoogleOption.Builder(Konfig.GOOGLE_SERVER_CLIENT_ID).build()
    val request = GetCredentialRequest.Builder().addCredentialOption(option).build()

    val credentialManager = CredentialManager.create(context)
    val result = credentialManager.getCredential(context, request)

    val googleIdTokenCredential = GoogleIdTokenCredential.createFrom(result.credential.data)

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.GOOGLE,
      params = mapOf("code" to googleIdTokenCredential.idToken),
    )
  }
}
