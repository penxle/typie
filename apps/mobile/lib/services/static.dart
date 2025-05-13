import 'dart:io';

import 'package:firebase_core/firebase_core.dart';
import 'package:in_app_purchase_storekit/in_app_purchase_storekit.dart';
import 'package:kakao_flutter_sdk_user/kakao_flutter_sdk_user.dart';
import 'package:naver_login_sdk/naver_login_sdk.dart';
import 'package:typie/env.dart';
import 'package:typie/firebase_options.dart';

Future<void> configureStaticServices() async {
  await Firebase.initializeApp(options: DefaultFirebaseOptions.currentPlatform);

  KakaoSdk.init(nativeAppKey: Env.kakaoNativeAppKey);

  await NaverLoginSDK.initialize(
    urlScheme: 'typie',
    clientName: '타이피',
    clientId: Env.naverClientId,
    clientSecret: Env.naverClientSecret,
  );

  if (Platform.isIOS) {
    await InAppPurchaseStoreKitPlatform.enableStoreKit2();
  }
}
