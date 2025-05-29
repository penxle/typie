import 'package:jiffy/jiffy.dart';

extension JiffyExtension on Jiffy {
  String get yyyyMMdd => format(pattern: 'yyyy.MM.dd');
  String get yyyyMMddKorean => format(pattern: 'yyyy년 MM월 dd일');
}
