import 'package:flutter/material.dart';

import '../theme/neobrutalist_theme.dart';

class ShortcutBar extends StatelessWidget {
  final ValueChanged<String> onShortcutPressed;

  const ShortcutBar({super.key, required this.onShortcutPressed});

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
    {'label': '🖥️ Monitor', 'id': 'switch_monitor'},
    {'label': '🔒 Privacy', 'id': 'privacy_mode'},
    {'label': '🔒 Lock PC', 'id': 'lock_pc'},
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
            return Padding(
              padding: const EdgeInsets.symmetric(horizontal: 3),
              child: Material(
                color: Colors.transparent,
                child: InkWell(
                  borderRadius: BorderRadius.circular(4),
                  onTap: () => onShortcutPressed(item['id']!),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 6,
                    ),
                    decoration: NeobrutalTheme.compactPanel(
                      color: NeobrutalTheme.surface,
                      shadow: true,
                    ),
                    child: Text(
                      item['label']!,
                      style: const TextStyle(
                        color: NeobrutalTheme.ink,
                        fontSize: 12,
                        fontWeight: FontWeight.w900,
                      ),
                    ),
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
