import 'package:get_it/get_it.dart';
import 'package:injectable/injectable.dart';
import 'package:typie/service.config.dart';

final serviceLocator = GetIt.instance;

@InjectableInit()
Future<void> configureServices() => serviceLocator.init();
