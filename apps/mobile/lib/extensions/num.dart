import 'package:intl/intl.dart';

final formatter = NumberFormat('#,##0.#');

extension NumExtension on num {
  String get comma => formatter.format(this);

  String get humanize {
    if (this <= 999) return comma;

    final (divisor, unit) = switch (this) {
      < 10000 => (1000.0, '천'),
      < 1e8 => (10000.0, '만'),
      < 1e12 => (1e8, '억'),
      _ => (1e12, '조'),
    };

    final value = ((this / divisor) * 10).floor() / 10;
    return '${value.comma}$unit';
  }
}
