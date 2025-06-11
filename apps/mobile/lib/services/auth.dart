import 'dart:async';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/material.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:gql_dio_link/gql_dio_link.dart';
import 'package:hive_ce/hive.dart';
import 'package:injectable/injectable.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/env.dart';
import 'package:typie/graphql/auth_link.dart';
import 'package:typie/services/__generated__/auth_query.data.gql.dart';
import 'package:typie/services/__generated__/auth_query.req.gql.dart';
import 'package:typie/services/kv.dart';
import 'package:typie/services/preference.dart';

part 'auth.freezed.dart';

@freezed
sealed class AuthState with _$AuthState {
  const AuthState._();

  const factory AuthState.initializing() = AuthInitializing;

  const factory AuthState.authenticated({
    required String sessionToken,
    required String accessToken,
    required GAuth_QueryData_me me,
  }) = Authenticated;

  const factory AuthState.unauthenticated() = Unauthenticated;
}

@singleton
class Auth extends ValueNotifier<AuthState> {
  Auth._(this._box, this._dio, this._pref, this._mixpanel) : super(const AuthState.initializing());

  final Box<dynamic> _box;
  final Dio _dio;
  final Pref _pref;
  final Mixpanel _mixpanel;

  final _sessionTokenKey = 'session_token';
  final _accessTokenKey = 'access_token';

  @FactoryMethod(preResolve: true)
  static Future<Auth> create(KV hive, Dio dio, Pref pref, Mixpanel mixpanel) async {
    final box = await hive.openBox('auth_box', encrypted: true);

    final auth = Auth._(box, dio, pref, mixpanel);
    await auth._refreshTokens();

    return auth;
  }

  Future<void> _refreshTokens() async {
    try {
      final sessionToken = _box.get(_sessionTokenKey) as String?;
      var accessToken = _box.get(_accessTokenKey) as String?;

      if (sessionToken == null) {
        throw Exception('No session token');
      }

      if (accessToken == null) {
        accessToken = await _getAccessToken(sessionToken);
        await _box.put(_accessTokenKey, accessToken);
      }

      final me = await _validateAccessToken(accessToken);

      _pref.siteId = me.sites.first.id;

      unawaited(_mixpanel.identify(me.id));
      _mixpanel.getPeople().set(r'$email', me.email);
      _mixpanel.getPeople().set(r'$name', me.name);
      _mixpanel.getPeople().set(r'$avatar', '${me.avatar.url}?s=256&f=png');

      value = AuthState.authenticated(sessionToken: sessionToken, accessToken: accessToken, me: me);
    } catch (_) {
      await clearTokens();
    }
  }

  Future<void> login(String sessionToken) async {
    await _box.put(_sessionTokenKey, sessionToken);

    await _refreshTokens();
  }

  Future<void> logout() async {
    final sessionToken = _box.get(_sessionTokenKey);

    if (sessionToken != null) {
      try {
        await _dio.get<void>(
          '${Env.authUrl}/logout',
          queryParameters: {'redirect_uri': 'typie:///'},
          options: Options(
            headers: {'Cookie': 'typie-st=$sessionToken'},
            followRedirects: false,
            validateStatus: (status) => status == 302,
          ),
        );
      } catch (_) {
        // pass
      }
    }

    await clearTokens();
  }

  Future<void> clearTokens() async {
    await _box.deleteAll([_sessionTokenKey, _accessTokenKey]);

    value = const AuthState.unauthenticated();

    unawaited(_mixpanel.reset());
  }

  Future<String> _getAccessToken(String sessionToken) async {
    final authorizeResponse = await _dio.get<void>(
      '${Env.authUrl}/authorize',
      queryParameters: {
        'response_type': 'code',
        'redirect_uri': 'typie:///authorize',
        'client_id': Env.oidcClientId,
        'prompt': 'none',
      },
      options: Options(
        headers: {'Cookie': 'typie-st=$sessionToken'},
        followRedirects: false,
        validateStatus: (status) => status == 302,
      ),
    );

    final uri = Uri.parse(authorizeResponse.headers.value(HttpHeaders.locationHeader)!);

    final error = uri.queryParameters['error'];
    if (error != null) {
      throw Exception('Authorize error: $error');
    }

    final code = uri.queryParameters['code'];
    if (code == null) {
      throw Exception('No code returned');
    }

    final tokenResponse = await _dio.post<Map<String, dynamic>>(
      '${Env.authUrl}/token',
      data: {
        'code': code,
        'grant_type': 'authorization_code',
        'redirect_uri': 'typie:///authorize',
        'client_id': Env.oidcClientId,
        'client_secret': Env.oidcClientSecret,
      },
      options: Options(contentType: Headers.formUrlEncodedContentType),
    );

    final accessToken = tokenResponse.data?['access_token'] as String?;
    if (accessToken == null) {
      throw Exception('No access token returned');
    }

    return accessToken;
  }

  Future<GAuth_QueryData_me> _validateAccessToken(String accessToken) async {
    final client = Client(
      link: Link.from([authLink(getAccessToken: () => accessToken), DioLink('${Env.apiUrl}/graphql', client: _dio)]),
    );

    final result = await client.request(GAuth_QueryReq()).first;
    if (result.data?.me == null) {
      throw Exception('Invalid access token');
    }

    return result.data!.me!;
  }
}
