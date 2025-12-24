import 'package:freezed_annotation/freezed_annotation.dart';

part 'bootstrap.freezed.dart';
part 'bootstrap.g.dart';

@freezed
sealed class Bootstrap with _$Bootstrap {
  const factory Bootstrap({
    required int version,
    required DateTime updatedAt,
    required MaintenanceConfig maintenance,
    required MinVersionConfig minVersion,
  }) = _Bootstrap;

  factory Bootstrap.fromJson(Map<String, dynamic> json) => _$BootstrapFromJson(json);
}

@freezed
sealed class MaintenanceConfig with _$MaintenanceConfig {
  const factory MaintenanceConfig({
    required bool enabled,
    required String title,
    required String message,
    DateTime? until,
    required List<String> platforms,
  }) = _MaintenanceConfig;

  factory MaintenanceConfig.fromJson(Map<String, dynamic> json) => _$MaintenanceConfigFromJson(json);
}

@freezed
sealed class MinVersionConfig with _$MinVersionConfig {
  const factory MinVersionConfig({required PlatformMinVersion ios, required PlatformMinVersion android}) =
      _MinVersionConfig;

  factory MinVersionConfig.fromJson(Map<String, dynamic> json) => _$MinVersionConfigFromJson(json);
}

@freezed
sealed class PlatformMinVersion with _$PlatformMinVersion {
  const factory PlatformMinVersion({required String version, required String storeUrl}) = _PlatformMinVersion;

  factory PlatformMinVersion.fromJson(Map<String, dynamic> json) => _$PlatformMinVersionFromJson(json);
}
