import 'dart:ui';
import 'package:flutter/material.dart';

class ShortcutBar extends StatelessWidget {
  final ValueChanged<String> onShortcutPressed;

  const ShortcutBar({
    super.key,
    required this.onShortcutPressed,
  });

  static const List<Map<String, String>> _shortcuts = [
    {'label': 'Win', 'id': 'win'},
    {'label': 'Esc', 'id': 'esc'},
    {'label': 'Enter', 'id': 'enter'},
    {'label': 'Del', 'id': 'backspace'},
    {'label': 'Tab', 'id': 'tab'},
    {'label': 'TaskMgr', 'id': 'task_manager'},
    {'label': 'Alt+Tab', 'id': 'alt_tab'},
    {'label': 'Desktop', 'id': 'show_desktop'},
    {'label': 'Ctrl+Alt+Del', 'id': 'ctrl_alt_del'},
    {'label': '🛡️ Elevate', 'id': 'elevate'},
  ];

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
          decoration: BoxDecoration(
            color: Colors.black87,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: Colors.white24, width: 1),
          ),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: _shortcuts.map((item) {
                return Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(8),
                    onTap: () => onShortcutPressed(item['id']!),
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 6,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.white.withValues(alpha: 0.1),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: Colors.white12),
                      ),
                      child: Text(
                        item['label']!,
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
          ),
        ),
      ),
    );
  }
}
