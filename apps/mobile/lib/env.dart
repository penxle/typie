import 'package:envied/envied.dart';

part 'env.g.dart';

@Envied(useConstantCase: true, obfuscate: true, environment: true)
abstract class Env {
  @EnviedField()
  static String authUrl = _Env.authUrl;

  @EnviedField()
  static String apiUrl = _Env.apiUrl;

  @EnviedField()
  static String oidcClientId = _Env.oidcClientId;

  @EnviedField()
  static String oidcClientSecret = _Env.oidcClientSecret;
}
