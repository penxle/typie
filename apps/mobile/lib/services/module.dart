import 'package:dio/dio.dart';
import 'package:dio_http2_adapter/dio_http2_adapter.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:google_sign_in/google_sign_in.dart';
import 'package:injectable/injectable.dart';
import 'package:typie/env.dart';

@module
abstract class RegisterModule {
  @singleton
  FlutterSecureStorage get flutterSecureStorage =>
      const FlutterSecureStorage(aOptions: AndroidOptions(encryptedSharedPreferences: true, resetOnError: true));

  @singleton
  Dio get dio => Dio()..httpClientAdapter = Http2Adapter(ConnectionManager());

  @singleton
  GoogleSignIn get googleSignIn => GoogleSignIn(
    clientId: Env.googleClientId,
    serverClientId: Env.googleServerClientId,
    scopes: ['email', 'profile'],
  );
}
