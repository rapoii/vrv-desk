import 'package:flutter/material.dart';

import '../theme/neobrutalist_theme.dart';

/// Unified 100% Virtual PC Keyboard for VrV Desk.
///
/// Combines alphanumeric typing, PC navigation/function keys, and sticky modifiers
/// into a single docked panel with dual operational modes:
/// 1. Direct Mode: keystrokes and shortcuts are immediately sent to the remote PC.
/// 2. Buffered Mode: text is accumulated inside a mobile preview box first; tapping
///    Send transmits the entire string to the PC at once.
class UnifiedVirtualKeyboard extends StatefulWidget {
  final ValueChanged<String> onTextInput;
  final ValueChanged<String> onShortcut;
  final VoidCallback onClose;
  final bool initialDirectMode;

  const UnifiedVirtualKeyboard({
    super.key,
    required this.onTextInput,
    required this.onShortcut,
    required this.onClose,
    this.initialDirectMode = false,
  });

  @override
  State<UnifiedVirtualKeyboard> createState() => _UnifiedVirtualKeyboardState();
}

class _UnifiedVirtualKeyboardState extends State<UnifiedVirtualKeyboard> {
  late bool _isDirectMode;
  bool _isFnLayer = false;

  // Sticky modifier states
  bool _isCtrlActive = false;
  bool _isAltActive = false;
  bool _isShiftActive = false;
  bool _isWinActive = false;

  final TextEditingController _bufferController = TextEditingController();
  final FocusNode _bufferFocusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _isDirectMode = widget.initialDirectMode;
  }

  @override
  void dispose() {
    _bufferController.dispose();
    _bufferFocusNode.dispose();
    super.dispose();
  }

  void _handleChar(String char) {
    if (_isDirectMode) {
      if (_isCtrlActive) {
        final lower = char.toLowerCase();
        if (lower == 'c') {
          widget.onShortcut('ctrl_c');
        } else if (lower == 'v') {
          widget.onShortcut('ctrl_v');
        } else if (lower == 'x') {
          widget.onShortcut('ctrl_x');
        } else if (lower == 'z') {
          widget.onShortcut('ctrl_z');
        } else if (lower == 'a') {
          widget.onShortcut('ctrl_a');
        } else {
          widget.onTextInput(char);
        }
        setState(() => _isCtrlActive = false);
        return;
      }

      widget.onTextInput(char);
      if (_isShiftActive) {
        setState(() => _isShiftActive = false);
      }
    } else {
      final currentText = _bufferController.text;
      final selection = _bufferController.selection;
      if (selection.isValid && selection.start >= 0) {
        final newText = currentText.replaceRange(
          selection.start,
          selection.end,
          char,
        );
        _bufferController.value = TextEditingValue(
          text: newText,
          selection: TextSelection.collapsed(
            offset: selection.start + char.length,
          ),
        );
      } else {
        _bufferController.text = currentText + char;
        _bufferController.selection = TextSelection.collapsed(
          offset: _bufferController.text.length,
        );
      }
      if (_isShiftActive) {
        setState(() => _isShiftActive = false);
      }
    }
  }

  void _handleBackspace() {
    if (_isDirectMode) {
      widget.onShortcut('backspace');
    } else {
      final text = _bufferController.text;
      if (text.isEmpty) return;

      final selection = _bufferController.selection;
      if (selection.isValid && selection.start > 0) {
        if (selection.start != selection.end) {
          final newText = text.replaceRange(selection.start, selection.end, '');
          _bufferController.value = TextEditingValue(
            text: newText,
            selection: TextSelection.collapsed(offset: selection.start),
          );
        } else {
          final newText = text.replaceRange(
            selection.start - 1,
            selection.start,
            '',
          );
          _bufferController.value = TextEditingValue(
            text: newText,
            selection: TextSelection.collapsed(offset: selection.start - 1),
          );
        }
      } else {
        _bufferController.text = text.substring(0, text.length - 1);
        _bufferController.selection = TextSelection.collapsed(
          offset: _bufferController.text.length,
        );
      }
    }
  }

  void _handleEnter() {
    if (_isDirectMode) {
      widget.onShortcut('enter');
    } else {
      // In buffered mode, Enter sends the buffered text to remote
      _submitBuffer();
    }
  }

  void _submitBuffer() {
    final text = _bufferController.text;
    if (text.isNotEmpty) {
      widget.onTextInput(text);
      _bufferController.clear();
      setState(() {});
    }
  }

  void _handleSpace() {
    if (_isDirectMode) {
      widget.onShortcut('space');
    } else {
      _handleChar(' ');
    }
  }

  void _triggerShortcut(String name) {
    widget.onShortcut(name);
  }

  Widget _buildKey({
    String? label,
    IconData? icon,
    double iconSize = 15,
    required VoidCallback onTap,
    Color? bg,
    Color? fg,
    int flex = 1,
    double fontSize = 13,
    bool isActive = false,
    Key? key,
  }) {
    final effectiveBg = isActive
        ? (bg ?? NeobrutalTheme.mint)
        : (bg ?? NeobrutalTheme.surface);
    final effectiveFg = fg ?? NeobrutalTheme.ink;

    Widget content;
    if (icon != null && label != null) {
      content = Row(
        mainAxisSize: MainAxisSize.min,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: iconSize, color: effectiveFg),
          const SizedBox(width: 2),
          Text(
            label,
            textAlign: TextAlign.center,
            style: TextStyle(
              color: effectiveFg,
              fontSize: fontSize,
              fontWeight: FontWeight.w900,
              letterSpacing: -0.2,
            ),
          ),
        ],
      );
    } else if (icon != null) {
      content = Icon(icon, size: iconSize, color: effectiveFg);
    } else {
      content = Text(
        label ?? '',
        textAlign: TextAlign.center,
        style: TextStyle(
          color: effectiveFg,
          fontSize: fontSize,
          fontWeight: FontWeight.w900,
          letterSpacing: -0.2,
        ),
      );
    }

    return Expanded(
      flex: flex,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 1.5, vertical: 2.0),
        child: NeobrutalPressable(
          key: key,
          onTap: onTap,
          cornerRadius: 4,
          shadowOffset: isActive ? const Offset(1, 1) : const Offset(2, 2),
          child: Container(
            height: 38,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              color: effectiveBg,
              borderRadius: BorderRadius.circular(4),
              border: Border.all(
                color: NeobrutalTheme.ink,
                width: NeobrutalTheme.compactBorderWidth,
              ),
            ),
            child: content,
          ),
        ),
      ),
    );
  }

  Widget _buildTopModeBar() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: const Color(0xFFEADBCE),
        border: const Border(
          bottom: BorderSide(
            color: NeobrutalTheme.ink,
            width: NeobrutalTheme.compactBorderWidth,
          ),
        ),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              // Segmented Mode Toggle: Direct vs Buffered
              Container(
                decoration: BoxDecoration(
                  color: NeobrutalTheme.paper,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: NeobrutalTheme.ink,
                    width: NeobrutalTheme.compactBorderWidth,
                  ),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    GestureDetector(
                      key: const Key('keyboard_mode_toggle_direct'),
                      onTap: () => setState(() => _isDirectMode = true),
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 4,
                        ),
                        decoration: BoxDecoration(
                          color: _isDirectMode
                              ? NeobrutalTheme.yellow
                              : Colors.transparent,
                          borderRadius: BorderRadius.circular(2),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: const [
                            Icon(Icons.bolt, size: 14, color: NeobrutalTheme.ink),
                            SizedBox(width: 3),
                            Text(
                              'Direct',
                              style: TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w900,
                                color: NeobrutalTheme.ink,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    GestureDetector(
                      key: const Key('keyboard_mode_toggle_buffered'),
                      onTap: () => setState(() => _isDirectMode = false),
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 4,
                        ),
                        decoration: BoxDecoration(
                          color: !_isDirectMode
                              ? NeobrutalTheme.cyan
                              : Colors.transparent,
                          borderRadius: BorderRadius.circular(2),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: const [
                            Icon(Icons.send_rounded, size: 12, color: NeobrutalTheme.ink),
                            SizedBox(width: 3),
                            Text(
                              'Send Mode',
                              style: TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w900,
                                color: NeobrutalTheme.ink,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 6),

              // Layer Toggle: QWERTY vs PC Function
              NeobrutalPressable(
                key: const Key('keyboard_layer_toggle_fn'),
                cornerRadius: 4,
                shadowOffset: const Offset(2, 2),
                onTap: () => setState(() => _isFnLayer = !_isFnLayer),
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 4,
                  ),
                  decoration: BoxDecoration(
                    color: _isFnLayer
                        ? NeobrutalTheme.pink
                        : NeobrutalTheme.surface,
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: NeobrutalTheme.ink,
                      width: NeobrutalTheme.compactBorderWidth,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        _isFnLayer ? Icons.keyboard : Icons.terminal,
                        size: 13,
                        color: NeobrutalTheme.ink,
                      ),
                      const SizedBox(width: 3),
                      Text(
                        _isFnLayer ? 'QWERTY' : 'PC Keys',
                        style: const TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w900,
                          color: NeobrutalTheme.ink,
                        ),
                      ),
                    ],
                  ),
                ),
              ),

              const Spacer(),

              // Close Dock Button
              NeobrutalPressable(
                key: const Key('keyboard_close_button'),
                cornerRadius: 4,
                shadowOffset: const Offset(2, 2),
                onTap: widget.onClose,
                child: Container(
                  padding: const EdgeInsets.all(4),
                  decoration: BoxDecoration(
                    color: NeobrutalTheme.danger,
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: NeobrutalTheme.ink,
                      width: NeobrutalTheme.compactBorderWidth,
                    ),
                  ),
                  child: const Icon(
                    Icons.close,
                    size: 14,
                    color: NeobrutalTheme.ink,
                  ),
                ),
              ),
            ],
          ),

          // In Buffered Mode: Show Input Text Box and Send Button
          if (!_isDirectMode) ...[
            const SizedBox(height: 6),
            Row(
              children: [
                Expanded(
                  child: Container(
                    height: 34,
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    decoration: BoxDecoration(
                      color: NeobrutalTheme.surface,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: NeobrutalTheme.ink,
                        width: NeobrutalTheme.compactBorderWidth,
                      ),
                    ),
                    alignment: Alignment.centerLeft,
                    child: TextField(
                      key: const Key('keyboard_buffer_text_field'),
                      controller: _bufferController,
                      focusNode: _bufferFocusNode,
                      style: const TextStyle(
                        fontSize: 13,
                        fontWeight: FontWeight.w700,
                        color: NeobrutalTheme.ink,
                      ),
                      decoration: const InputDecoration(
                        isDense: true,
                        hintText: 'Ketik di sini lalu tekan Send...',
                        hintStyle: TextStyle(
                          fontSize: 12,
                          color: NeobrutalTheme.muted,
                        ),
                        border: InputBorder.none,
                        contentPadding: EdgeInsets.zero,
                      ),
                      onSubmitted: (_) => _submitBuffer(),
                    ),
                  ),
                ),
                const SizedBox(width: 4),
                if (_bufferController.text.isNotEmpty)
                  NeobrutalPressable(
                    key: const Key('keyboard_buffer_clear_button'),
                    cornerRadius: 4,
                    shadowOffset: const Offset(2, 2),
                    onTap: () => setState(() => _bufferController.clear()),
                    child: Container(
                      height: 34,
                      width: 32,
                      decoration: BoxDecoration(
                        color: NeobrutalTheme.paper,
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(
                          color: NeobrutalTheme.ink,
                          width: NeobrutalTheme.compactBorderWidth,
                        ),
                      ),
                      child: const Icon(
                        Icons.clear,
                        size: 16,
                        color: NeobrutalTheme.ink,
                      ),
                    ),
                  ),
                const SizedBox(width: 4),
                NeobrutalPressable(
                  key: const Key('keyboard_buffer_send_button'),
                  cornerRadius: 4,
                  shadowOffset: const Offset(2, 2),
                  onTap: _submitBuffer,
                  child: Container(
                    height: 34,
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    decoration: BoxDecoration(
                      color: NeobrutalTheme.mint,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: NeobrutalTheme.ink,
                        width: NeobrutalTheme.compactBorderWidth,
                      ),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: const [
                        Icon(
                          Icons.send,
                          size: 14,
                          color: NeobrutalTheme.ink,
                        ),
                        SizedBox(width: 4),
                        Text(
                          'SEND',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w900,
                            color: NeobrutalTheme.ink,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildQwertyLayer() {
    final shift = _isShiftActive;

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Helper / Quick modifier row
        Row(
          children: [
            _buildKey(
              label: 'ESC',
              bg: NeobrutalTheme.yellow,
              fontSize: 11,
              onTap: () => _triggerShortcut('esc'),
            ),
            _buildKey(
              label: 'TAB',
              fontSize: 11,
              onTap: () => _triggerShortcut('tab'),
            ),
            _buildKey(
              label: 'CTRL',
              fontSize: 11,
              isActive: _isCtrlActive,
              onTap: () => setState(() => _isCtrlActive = !_isCtrlActive),
            ),
            _buildKey(
              label: 'ALT',
              fontSize: 11,
              isActive: _isAltActive,
              onTap: () => setState(() => _isAltActive = !_isAltActive),
            ),
            _buildKey(
              label: 'WIN',
              icon: Icons.window,
              iconSize: 12,
              fontSize: 10,
              isActive: _isWinActive,
              onTap: () => _triggerShortcut('win'),
            ),
            _buildKey(
              label: 'DEL',
              fontSize: 11,
              bg: NeobrutalTheme.danger,
              onTap: () => _triggerShortcut('del'),
            ),
            _buildKey(
              key: const Key('keyboard_key_backspace'),
              icon: Icons.backspace_outlined,
              iconSize: 16,
              flex: 2,
              bg: const Color(0xFFFFD5D5),
              onTap: _handleBackspace,
            ),
          ],
        ),

        // Row 1: Numbers / Symbols
        Row(
          children: [
            _buildKey(label: shift ? '!' : '1', onTap: () => _handleChar(shift ? '!' : '1')),
            _buildKey(label: shift ? '@' : '2', onTap: () => _handleChar(shift ? '@' : '2')),
            _buildKey(label: shift ? '#' : '3', onTap: () => _handleChar(shift ? '#' : '3')),
            _buildKey(label: shift ? '\$' : '4', onTap: () => _handleChar(shift ? '\$' : '4')),
            _buildKey(label: shift ? '%' : '5', onTap: () => _handleChar(shift ? '%' : '5')),
            _buildKey(label: shift ? '^' : '6', onTap: () => _handleChar(shift ? '^' : '6')),
            _buildKey(label: shift ? '&' : '7', onTap: () => _handleChar(shift ? '&' : '7')),
            _buildKey(label: shift ? '*' : '8', onTap: () => _handleChar(shift ? '*' : '8')),
            _buildKey(label: shift ? '(' : '9', onTap: () => _handleChar(shift ? '(' : '9')),
            _buildKey(label: shift ? ')' : '0', onTap: () => _handleChar(shift ? ')' : '0')),
            _buildKey(label: shift ? '_' : '-', onTap: () => _handleChar(shift ? '_' : '-')),
            _buildKey(label: shift ? '+' : '=', onTap: () => _handleChar(shift ? '+' : '=')),
          ],
        ),

        // Row 2: QWERTYUIOP
        Row(
          children: [
            _buildKey(label: shift ? 'Q' : 'q', onTap: () => _handleChar(shift ? 'Q' : 'q')),
            _buildKey(label: shift ? 'W' : 'w', onTap: () => _handleChar(shift ? 'W' : 'w')),
            _buildKey(label: shift ? 'E' : 'e', onTap: () => _handleChar(shift ? 'E' : 'e')),
            _buildKey(label: shift ? 'R' : 'r', onTap: () => _handleChar(shift ? 'R' : 'r')),
            _buildKey(label: shift ? 'T' : 't', onTap: () => _handleChar(shift ? 'T' : 't')),
            _buildKey(label: shift ? 'Y' : 'y', onTap: () => _handleChar(shift ? 'Y' : 'y')),
            _buildKey(label: shift ? 'U' : 'u', onTap: () => _handleChar(shift ? 'U' : 'u')),
            _buildKey(label: shift ? 'I' : 'i', onTap: () => _handleChar(shift ? 'I' : 'i')),
            _buildKey(label: shift ? 'O' : 'o', onTap: () => _handleChar(shift ? 'O' : 'o')),
            _buildKey(label: shift ? 'P' : 'p', onTap: () => _handleChar(shift ? 'P' : 'p')),
            _buildKey(label: shift ? '{' : '[', onTap: () => _handleChar(shift ? '{' : '[')),
            _buildKey(label: shift ? '}' : ']', onTap: () => _handleChar(shift ? '}' : ']')),
          ],
        ),

        // Row 3: ASDFGHJKL
        Row(
          children: [
            _buildKey(label: shift ? 'A' : 'a', onTap: () => _handleChar(shift ? 'A' : 'a')),
            _buildKey(label: shift ? 'S' : 's', onTap: () => _handleChar(shift ? 'S' : 's')),
            _buildKey(label: shift ? 'D' : 'd', onTap: () => _handleChar(shift ? 'D' : 'd')),
            _buildKey(label: shift ? 'F' : 'f', onTap: () => _handleChar(shift ? 'F' : 'f')),
            _buildKey(label: shift ? 'G' : 'g', onTap: () => _handleChar(shift ? 'G' : 'g')),
            _buildKey(label: shift ? 'H' : 'h', onTap: () => _handleChar(shift ? 'H' : 'h')),
            _buildKey(label: shift ? 'J' : 'j', onTap: () => _handleChar(shift ? 'J' : 'j')),
            _buildKey(label: shift ? 'K' : 'k', onTap: () => _handleChar(shift ? 'K' : 'k')),
            _buildKey(label: shift ? 'L' : 'l', onTap: () => _handleChar(shift ? 'L' : 'l')),
            _buildKey(label: shift ? ':' : ';', onTap: () => _handleChar(shift ? ':' : ';')),
            _buildKey(label: shift ? '"' : "'", onTap: () => _handleChar(shift ? '"' : "'")),
          ],
        ),

        // Row 4: Shift ZXCVBNM
        Row(
          children: [
            _buildKey(
              key: const Key('keyboard_key_shift'),
              icon: Icons.arrow_upward,
              iconSize: 16,
              flex: 2,
              isActive: _isShiftActive,
              bg: NeobrutalTheme.yellow,
              onTap: () => setState(() => _isShiftActive = !_isShiftActive),
            ),
            _buildKey(label: shift ? 'Z' : 'z', onTap: () => _handleChar(shift ? 'Z' : 'z')),
            _buildKey(label: shift ? 'X' : 'x', onTap: () => _handleChar(shift ? 'X' : 'x')),
            _buildKey(label: shift ? 'C' : 'c', onTap: () => _handleChar(shift ? 'C' : 'c')),
            _buildKey(label: shift ? 'V' : 'v', onTap: () => _handleChar(shift ? 'V' : 'v')),
            _buildKey(label: shift ? 'B' : 'b', onTap: () => _handleChar(shift ? 'B' : 'b')),
            _buildKey(label: shift ? 'N' : 'n', onTap: () => _handleChar(shift ? 'N' : 'n')),
            _buildKey(label: shift ? 'M' : 'm', onTap: () => _handleChar(shift ? 'M' : 'm')),
            _buildKey(label: shift ? '<' : ',', onTap: () => _handleChar(shift ? '<' : ',')),
            _buildKey(label: shift ? '>' : '.', onTap: () => _handleChar(shift ? '>' : '.')),
            _buildKey(label: shift ? '?' : '/', onTap: () => _handleChar(shift ? '?' : '/')),
            _buildKey(
              key: const Key('keyboard_key_enter'),
              icon: Icons.keyboard_return,
              iconSize: 16,
              flex: 2,
              bg: NeobrutalTheme.cyan,
              onTap: _handleEnter,
            ),
          ],
        ),

        // Row 5: Space & Navigation
        Row(
          children: [
            _buildKey(
              label: 'Fn',
              bg: NeobrutalTheme.pink,
              fontSize: 11,
              onTap: () => setState(() => _isFnLayer = true),
            ),
            _buildKey(
              label: 'Ctrl',
              fontSize: 11,
              isActive: _isCtrlActive,
              onTap: () => setState(() => _isCtrlActive = !_isCtrlActive),
            ),
            _buildKey(
              label: 'Alt',
              fontSize: 11,
              isActive: _isAltActive,
              onTap: () => setState(() => _isAltActive = !_isAltActive),
            ),
            _buildKey(
              label: 'SPACE',
              flex: 5,
              fontSize: 11,
              onTap: _handleSpace,
            ),
            _buildKey(
              icon: Icons.arrow_left,
              iconSize: 18,
              onTap: () => _triggerShortcut('left'),
            ),
            _buildKey(
              icon: Icons.arrow_drop_up,
              iconSize: 18,
              onTap: () => _triggerShortcut('up'),
            ),
            _buildKey(
              icon: Icons.arrow_drop_down,
              iconSize: 18,
              onTap: () => _triggerShortcut('down'),
            ),
            _buildKey(
              icon: Icons.arrow_right,
              iconSize: 18,
              onTap: () => _triggerShortcut('right'),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildFnLayer() {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Function keys Row 1: F1 - F6
        Row(
          children: [
            _buildKey(
              label: 'ESC',
              bg: NeobrutalTheme.yellow,
              fontSize: 11,
              onTap: () => _triggerShortcut('esc'),
            ),
            _buildKey(label: 'F1', fontSize: 11, onTap: () => _triggerShortcut('f1')),
            _buildKey(label: 'F2', fontSize: 11, onTap: () => _triggerShortcut('f2')),
            _buildKey(label: 'F3', fontSize: 11, onTap: () => _triggerShortcut('f3')),
            _buildKey(label: 'F4', fontSize: 11, onTap: () => _triggerShortcut('f4')),
            _buildKey(label: 'F5', fontSize: 11, bg: NeobrutalTheme.mint, onTap: () => _triggerShortcut('f5')),
            _buildKey(label: 'F6', fontSize: 11, onTap: () => _triggerShortcut('f6')),
            _buildKey(
              icon: Icons.backspace_outlined,
              iconSize: 16,
              flex: 2,
              bg: const Color(0xFFFFD5D5),
              onTap: _handleBackspace,
            ),
          ],
        ),

        // Function keys Row 2: F7 - F12
        Row(
          children: [
            _buildKey(
              label: 'TAB',
              fontSize: 11,
              onTap: () => _triggerShortcut('tab'),
            ),
            _buildKey(label: 'F7', fontSize: 11, onTap: () => _triggerShortcut('f7')),
            _buildKey(label: 'F8', fontSize: 11, onTap: () => _triggerShortcut('f8')),
            _buildKey(label: 'F9', fontSize: 11, onTap: () => _triggerShortcut('f9')),
            _buildKey(label: 'F10', fontSize: 11, onTap: () => _triggerShortcut('f10')),
            _buildKey(label: 'F11', fontSize: 11, onTap: () => _triggerShortcut('f11')),
            _buildKey(label: 'F12', fontSize: 11, onTap: () => _triggerShortcut('f12')),
            _buildKey(
              icon: Icons.keyboard_return,
              iconSize: 16,
              flex: 2,
              bg: NeobrutalTheme.cyan,
              onTap: _handleEnter,
            ),
          ],
        ),

        // Navigation cluster: Ins, Del, Home, End, PgUp, PgDn, PrtScn
        Row(
          children: [
            _buildKey(label: 'INS', fontSize: 11, onTap: () => _triggerShortcut('ins')),
            _buildKey(label: 'DEL', fontSize: 11, bg: NeobrutalTheme.danger, onTap: () => _triggerShortcut('del')),
            _buildKey(label: 'HOME', fontSize: 11, onTap: () => _triggerShortcut('home')),
            _buildKey(label: 'END', fontSize: 11, onTap: () => _triggerShortcut('end')),
            _buildKey(label: 'PGUP', fontSize: 11, onTap: () => _triggerShortcut('pgup')),
            _buildKey(label: 'PGDN', fontSize: 11, onTap: () => _triggerShortcut('pgdn')),
            _buildKey(label: 'PrtScn', fontSize: 10, onTap: () => _triggerShortcut('prtscn')),
          ],
        ),

        // System & Editing shortcuts
        Row(
          children: [
            _buildKey(
              label: 'Ctrl+Alt+Del',
              fontSize: 10,
              bg: NeobrutalTheme.danger,
              onTap: () => _triggerShortcut('ctrl_alt_del'),
            ),
            _buildKey(
              label: 'Alt+Tab',
              fontSize: 10,
              onTap: () => _triggerShortcut('alt_tab'),
            ),
            _buildKey(
              label: 'Win+D',
              fontSize: 10,
              onTap: () => _triggerShortcut('show_desktop'),
            ),
            _buildKey(
              label: 'Win+R',
              fontSize: 10,
              onTap: () => _triggerShortcut('win_r'),
            ),
            _buildKey(
              label: 'TaskMgr',
              fontSize: 10,
              onTap: () => _triggerShortcut('task_manager'),
            ),
          ],
        ),

        // Clipboard quick combos
        Row(
          children: [
            _buildKey(
              label: 'Ctrl+A',
              fontSize: 11,
              onTap: () => _triggerShortcut('ctrl_a'),
            ),
            _buildKey(
              label: 'Ctrl+C',
              fontSize: 11,
              onTap: () => _triggerShortcut('ctrl_c'),
            ),
            _buildKey(
              label: 'Ctrl+V',
              fontSize: 11,
              onTap: () => _triggerShortcut('ctrl_v'),
            ),
            _buildKey(
              label: 'Ctrl+X',
              fontSize: 11,
              onTap: () => _triggerShortcut('ctrl_x'),
            ),
            _buildKey(
              label: 'Ctrl+Z',
              fontSize: 11,
              onTap: () => _triggerShortcut('ctrl_z'),
            ),
          ],
        ),

        // Back to QWERTY + Arrows
        Row(
          children: [
            _buildKey(
              label: 'QWERTY Layout',
              icon: Icons.keyboard,
              iconSize: 14,
              flex: 4,
              bg: NeobrutalTheme.yellow,
              fontSize: 11,
              onTap: () => setState(() => _isFnLayer = false),
            ),
            _buildKey(
              icon: Icons.arrow_left,
              iconSize: 18,
              onTap: () => _triggerShortcut('left'),
            ),
            _buildKey(
              icon: Icons.arrow_drop_up,
              iconSize: 18,
              onTap: () => _triggerShortcut('up'),
            ),
            _buildKey(
              icon: Icons.arrow_drop_down,
              iconSize: 18,
              onTap: () => _triggerShortcut('down'),
            ),
            _buildKey(
              icon: Icons.arrow_right,
              iconSize: 18,
              onTap: () => _triggerShortcut('right'),
            ),
          ],
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const Key('unified_keyboard_dock'),
      decoration: BoxDecoration(
        color: NeobrutalTheme.paper,
        border: const Border(
          top: BorderSide(
            color: NeobrutalTheme.ink,
            width: NeobrutalTheme.borderWidth,
          ),
        ),
        boxShadow: const [
          BoxShadow(
            color: NeobrutalTheme.ink,
            offset: Offset(0, -3),
            blurRadius: 0,
          ),
        ],
      ),
      child: SafeArea(
        top: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            _buildTopModeBar(),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
              child: _isFnLayer ? _buildFnLayer() : _buildQwertyLayer(),
            ),
          ],
        ),
      ),
    );
  }
}
