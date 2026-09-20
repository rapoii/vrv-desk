import 'dart:math';

import 'package:flutter/material.dart';

import 'src/theme/neobrutalist_theme.dart';
import 'src/views/home_view.dart';

void main() {
  runApp(const VrvDeskApp());
}

class VrvDeskApp extends StatefulWidget {
  const VrvDeskApp({super.key});

  @override
  State<VrvDeskApp> createState() => _VrvDeskAppState();
}

class _VrvDeskAppState extends State<VrvDeskApp> {
  late final String _myDeviceId;

  @override
  void initState() {
    super.initState();
    // Deterministic or pseudo-random 6-digit Device ID for session
    _myDeviceId = (100000 + Random().nextInt(900000)).toString();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'VrV Desk',
      debugShowCheckedModeBanner: false,
      theme: NeobrutalTheme.light,
      darkTheme: NeobrutalTheme.light,
      themeMode: ThemeMode.light,
      home: HomeView(myDeviceId: _myDeviceId),
    );
  }
}
