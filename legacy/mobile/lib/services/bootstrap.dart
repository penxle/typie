import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:hive_ce/hive.dart';
import 'package:injectable/injectable.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:typie/env.dart';
import 'package:typie/models/bootstrap.dart';
import 'package:typie/services/kv.dart';

part 'bootstrap.freezed.dart';

@freezed
sealed class BootstrapState with _$BootstrapState {
  const factory BootstrapState.loading() = BootstrapLoading;
  const factory BootstrapState.maintenance({required String title, required String message, DateTime? until}) =
      BootstrapMaintenance;
  const factory BootstrapState.updateRequired({
    required String storeUrl,
    required String currentVersion,
    required String requiredVersion,
  }) = BootstrapUpdateRequired;
  const factory BootstrapState.ready() = BootstrapReady;
}

@singleton
class BootstrapService extends ValueNotifier<BootstrapState> {
  BootstrapService._(this._box, this._dio, this._packageInfo) : super(const BootstrapState.loading());

  final Box<dynamic> _box;
  final Dio _dio;
  final PackageInfo _packageInfo;
  Timer? _timer;

  static const _cacheKey = 'bootstrap_cache';
  static const _refreshInterval = Duration(minutes: 1);

  @FactoryMethod(preResolve: true)
  static Future<BootstrapService> create(KV hive, Dio dio, PackageInfo packageInfo) async {
    final box = await hive.openBox('bootstrap_box');
    final service = BootstrapService._(box, dio, packageInfo);
    await service._fetch();
    service._startPeriodicRefresh();
    return service;
  }

  void _startPeriodicRefresh() {
    _timer = Timer.periodic(_refreshInterval, (_) => _fetch());
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _fetch() async {
    try {
      final response = await _dio.get<String>(
        Env.bootstrapUrl,
        options: Options(
          responseType: ResponseType.plain,
          sendTimeout: const Duration(seconds: 10),
          receiveTimeout: const Duration(seconds: 10),
        ),
      );

      if (response.data != null) {
        await _box.put(_cacheKey, response.data);
        _processBootstrap(response.data!);
      }
    } on DioException {
      final cached = _box.get(_cacheKey) as String?;
      if (cached != null) {
        _processBootstrap(cached);
      } else {
        value = const BootstrapState.ready();
      }
    } catch (_) {
      value = const BootstrapState.ready();
    }
  }

  void _processBootstrap(String jsonString) {
    try {
      final json = jsonDecode(jsonString) as Map<String, dynamic>;
      final bootstrap = Bootstrap.fromJson(json);

      final currentPlatform = Platform.isIOS ? 'ios' : 'android';

      if (bootstrap.maintenance.enabled && bootstrap.maintenance.platforms.contains(currentPlatform)) {
        value = BootstrapState.maintenance(
          title: bootstrap.maintenance.title,
          message: bootstrap.maintenance.message,
          until: bootstrap.maintenance.until,
        );
        return;
      }

      final platformConfig = Platform.isIOS ? bootstrap.minVersion.ios : bootstrap.minVersion.android;
      if (_isVersionLower(_packageInfo.version, platformConfig.version)) {
        value = BootstrapState.updateRequired(
          storeUrl: platformConfig.storeUrl,
          currentVersion: _packageInfo.version,
          requiredVersion: platformConfig.version,
        );
        return;
      }

      value = const BootstrapState.ready();
    } catch (_) {
      value = const BootstrapState.ready();
    }
  }

  bool _isVersionLower(String current, String required) {
    final currentParts = current.split('.').map((e) => int.tryParse(e) ?? 0).toList();
    final requiredParts = required.split('.').map((e) => int.tryParse(e) ?? 0).toList();

    while (currentParts.length < 3) {
      currentParts.add(0);
    }
    while (requiredParts.length < 3) {
      requiredParts.add(0);
    }

    for (var i = 0; i < 3; i++) {
      if (currentParts[i] < requiredParts[i]) {
        return true;
      }
      if (currentParts[i] > requiredParts[i]) {
        return false;
      }
    }

    return false;
  }

  Future<void> refresh() async {
    value = const BootstrapState.loading();
    await _fetch();
  }
}
