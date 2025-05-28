import 'dart:async';
import 'dart:io';

import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/providers/__generated__/push_notification.req.gql.dart';
import 'package:typie/services/auth.dart';

class PushNotificationProvider extends HookWidget {
  const PushNotificationProvider({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final client = useService<GraphQLClient>();
    final authState = useValueListenable(auth);

    useEffect(() {
      if (authState is Authenticated) {
        unawaited(_registerToken(client));
      } else {
        unawaited(_deleteToken());
      }

      return null;
    }, [authState]);

    useEffect(() {
      final subscription = FirebaseMessaging.onMessage.listen((message) {
        final title = message.notification?.title;
        if (context.mounted && title != null) {
          context.toast(ToastType.notification, title, duration: const Duration(seconds: 10));
        }
      });

      return subscription.cancel;
    }, []);

    return const SizedBox.shrink();
  }
}

Future<void> _registerToken(GraphQLClient client) async {
  try {
    final status = await Permission.notification.request();
    if (!status.isGranted) {
      return;
    }

    if (Platform.isIOS) {
      final token = await FirebaseMessaging.instance.getAPNSToken();
      if (token == null) {
        return;
      }
    }

    final token = await FirebaseMessaging.instance.getToken();
    if (token == null) {
      return;
    }

    await client.request(
      GPushNotificationProvider_RegisterPushNotificationToken_MutationReq((b) => b..vars.input.token = token),
    );
  } catch (_) {
    // pass
  }
}

Future<void> _deleteToken() async {
  try {
    await FirebaseMessaging.instance.deleteToken();
  } catch (_) {
    // pass
  }
}
