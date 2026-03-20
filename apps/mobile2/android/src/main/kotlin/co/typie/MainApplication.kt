package co.typie

import android.app.Application
import co.typie.di.initKoin
import com.kakao.sdk.common.KakaoSdk
import com.navercorp.nid.NidOAuth
import org.koin.android.ext.koin.androidContext
import org.koin.android.ext.koin.androidLogger

class MainApplication : Application() {
  override fun onCreate() {
    super.onCreate()

    initKoin {
      androidContext(this@MainApplication)
      androidLogger()
    }

    KakaoSdk.init(this, Konfig.KAKAO_NATIVE_APP_KEY)
    NidOAuth.initialize(
      this,
      Konfig.NAVER_CLIENT_ID,
      Konfig.NAVER_CLIENT_SECRET,
      "타이피",
    )
  }
}