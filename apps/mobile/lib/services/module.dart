// ignore_for_file: discarded_futures static initializations

import 'package:dio/dio.dart';
import 'package:dio_http2_adapter/dio_http2_adapter.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:google_sign_in/google_sign_in.dart';
import 'package:injectable/injectable.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/env.dart';
import 'package:typie/styles/colors.dart';

@module
abstract class RegisterModule {
  @singleton
  FlutterSecureStorage get flutterSecureStorage =>
      const FlutterSecureStorage(aOptions: AndroidOptions(encryptedSharedPreferences: true, resetOnError: true));

  @preResolve
  @singleton
  Future<Mixpanel> get mixpanel => Mixpanel.init(Env.mixpanelToken, trackAutomaticEvents: false);

  @singleton
  Dio get dio => kDebugMode
      ? (Dio()..httpClientAdapter = HttpClientAdapter())
      : (Dio()..httpClientAdapter = Http2Adapter(ConnectionManager()));

  @singleton
  GoogleSignIn get googleSignIn => GoogleSignIn(
    clientId: Env.googleClientId,
    serverClientId: Env.googleServerClientId,
    scopes: ['email', 'profile'],
  );

  @singleton
  FlutterLocalNotificationsPlugin get flutterLocalNotificationsPlugin => FlutterLocalNotificationsPlugin()
    ..initialize(
      const InitializationSettings(
        android: AndroidInitializationSettings('@drawable/ic_notification_foreground'),
        iOS: DarwinInitializationSettings(
          requestAlertPermission: false,
          requestBadgePermission: false,
          requestSoundPermission: false,
        ),
      ),
    )
    ..resolvePlatformSpecificImplementation<AndroidFlutterLocalNotificationsPlugin>()?.createNotificationChannelGroup(
      const AndroidNotificationChannelGroup('default', '기본'),
    )
    ..resolvePlatformSpecificImplementation<AndroidFlutterLocalNotificationsPlugin>()?.createNotificationChannel(
      const AndroidNotificationChannel(
        'default',
        '기본 알림',
        groupId: 'default',
        importance: Importance.max,
        enableLights: true,
        ledColor: AppColors.brand_500,
      ),
    );
}
