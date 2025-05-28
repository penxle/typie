import 'package:jiffy/jiffy.dart';

extension JiffyExtension on Jiffy {
  String get yyyyMMdd => format(pattern: 'yyyy.MM.dd');
}
