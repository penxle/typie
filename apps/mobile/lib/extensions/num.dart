import 'package:intl/intl.dart';

final formatter = NumberFormat('#,###');

extension NumExtension on num {
  String get comma => formatter.format(this);
}
