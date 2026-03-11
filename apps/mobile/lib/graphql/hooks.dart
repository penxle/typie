import 'dart:async';

import 'package:auto_route/auto_route.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/routers/observer.dart';

TData? useQuery<TData, TVars>(OperationRequest<TData, TVars> operation) {
  final client = useService<GraphQLClient>();

  final stream = useMemoized(() => client.raw.request(operation).distinct(), [operation]);
  final snapshot = useStream(stream);

  useRefetchOnRouteResumed(operation);

  if (snapshot.data?.hasErrors ?? false) {
    // ignore: only_throw_errors -- it is an error
    throw snapshot.data!.linkException ?? snapshot.data!.graphqlErrors!.first;
  }

  return snapshot.data?.data;
}

void useRefetchOnRouteResumed<TData, TVars>(OperationRequest<TData, TVars> operation) {
  final client = useService<GraphQLClient>();
  final context = useContext();
  final routeDataScope = context.dependOnInheritedWidgetOfExactType<RouteDataScope>();
  final routerScope = RouterScope.of(context);
  final autoRouteObserver = routeDataScope != null ? routerScope.firstObserverOfType<AutoRouteObserver>() : null;
  final modalRoute = ModalRoute.of(context);
  final widgetRouteObserver = routeDataScope == null && modalRoute != null
      ? routerScope.firstObserverOfType<WidgetRouteObserver>()
      : null;
  final routeData = routeDataScope?.routeData;

  useEffect(() {
    if (autoRouteObserver != null && routeData != null) {
      final aware = _RefetchAutoRouteAware(
        onResume: () {
          unawaited(client.refetch(operation));
        },
      );

      autoRouteObserver.subscribe(aware, routeData);
      return () => autoRouteObserver.unsubscribe(aware);
    }

    if (widgetRouteObserver != null && modalRoute != null) {
      final aware = _RefetchWidgetRouteAware(
        onResume: () {
          unawaited(client.refetch(operation));
        },
      );

      widgetRouteObserver.subscribe(aware, modalRoute);
      return () => widgetRouteObserver.unsubscribe(aware);
    }

    return null;
  }, [autoRouteObserver, widgetRouteObserver, routeData, modalRoute, operation]);
}

class _RefetchAutoRouteAware with AutoRouteAware {
  _RefetchAutoRouteAware({required this.onResume});
  final VoidCallback onResume;

  @override
  void didPopNext() => onResume();

  @override
  void didChangeTabRoute(TabPageRoute previousRoute) => onResume();
}

class _RefetchWidgetRouteAware with RouteAware {
  _RefetchWidgetRouteAware({required this.onResume});
  final VoidCallback onResume;

  @override
  void didPopNext() => onResume();
}
