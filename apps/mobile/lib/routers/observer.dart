import 'dart:async';

import 'package:auto_route/auto_route.dart';
import 'package:firebase_analytics/firebase_analytics.dart';
import 'package:flutter/material.dart';
import 'package:typie/service.dart';

class RouterObserver extends AutoRouterObserver {
  final analytics = serviceLocator<FirebaseAnalytics>();

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    unawaited(analytics.logScreenView(screenName: route.settings.name));
  }

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    unawaited(analytics.logScreenView(screenName: previousRoute?.settings.name));
  }

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) {
    unawaited(analytics.logScreenView(screenName: newRoute?.settings.name));
  }

  @override
  void didChangeTabRoute(TabPageRoute route, TabPageRoute previousRoute) {
    unawaited(analytics.logScreenView(screenName: route.name));
  }
}
