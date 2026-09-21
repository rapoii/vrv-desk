import 'package:flutter/material.dart';

import '../theme/neobrutalist_theme.dart';

class ShortcutBar extends StatelessWidget {
  final ValueChanged<String> onShortcutPressed;

  const ShortcutBar({super.key, required this.onShortcutPressed});

  static const List<Map<String, dynamic>> _shortcuts = [
    {'label': 'Win', 'id': 'win', 'icon': Icons.window},
    {'label': 'Esc', 'id': 'esc'},
    {'label': 'Enter', 'id': 'enter', 'icon': Icons.keyboard_return},
    {'label': 'Del', 'id': 'backspace', 'icon': Icons.backspace_outlined},
    {'label': 'Tab', 'id': 'tab'},
    {'label': 'TaskMgr', 'id': 'task_manager', 'icon': Icons.terminal},
    {'label': 'Alt+Tab', 'id': 'alt_tab'},
    {'label': 'Desktop', 'id': 'show_desktop'},
    {'label': 'Ctrl+Alt+Del', 'id': 'ctrl_alt_del'},
    {'label': 'Elevate', 'id': 'elevate', 'icon': Icons.security},
    {'label': 'Monitor', 'id': 'switch_monitor', 'icon': Icons.desktop_windows},
    {'label': 'Privacy', 'id': 'privacy_mode', 'icon': Icons.visibility_off},
    {'label': 'Lock PC', 'id': 'lock_pc', 'icon': Icons.lock},
  ];

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: NeobrutalTheme.panel(
        color: NeobrutalTheme.paper,
        cornerRadius: NeobrutalTheme.radius,
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: _shortcuts.map((item) {
            final IconData? icon = item['icon'] as IconData?;
            final String label = item['label'] as String;
            final String id = item['id'] as String;

            return Padding(
              padding: const EdgeInsets.symmetric(horizontal: 3),
              child: NeobrutalPressable(
                cornerRadius: 4,
                shadowOffset: const Offset(2, 2),
                onTap: () => onShortcutPressed(id),
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 6,
                  ),
                  decoration: NeobrutalTheme.compactPanel(
                    color: NeobrutalTheme.surface,
                    shadow: false,
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (icon != null) ...[
                        Icon(
                          icon,
                          size: 13,
                          color: NeobrutalTheme.ink,
                        ),
                        const SizedBox(width: 4),
                      ],
                      Text(
                        label,
                        style: const TextStyle(
                          color: NeobrutalTheme.ink,
                          fontSize: 12,
                          fontWeight: FontWeight.w900,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            );
          }).toList(),
        ),
      ),
    );
  }
}
