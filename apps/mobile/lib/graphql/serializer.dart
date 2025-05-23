import 'dart:async';

import 'package:built_value/serializer.dart';
import 'package:jiffy/jiffy.dart';

class DateTimeSerializer implements PrimitiveSerializer<Jiffy> {
  DateTimeSerializer() {
    unawaited(Jiffy.setLocale('ko_KR'));
  }

  @override
  Jiffy deserialize(Serializers serializers, Object serialized, {FullType specifiedType = FullType.unspecified}) {
    assert(serialized is String, 'DateTimeSerializer expected String but got ${serialized.runtimeType}');
    return Jiffy.parse(serialized as String);
  }

  @override
  Object serialize(Serializers serializers, Jiffy object, {FullType specifiedType = FullType.unspecified}) {
    return object.format();
  }

  @override
  Iterable<Type> get types => [Jiffy];

  @override
  String get wireName => 'DateTime';
}
