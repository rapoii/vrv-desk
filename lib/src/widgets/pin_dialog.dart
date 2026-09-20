import 'package:flutter/material.dart';

class PinDialog extends StatefulWidget {
  final String deviceName;
  final ValueChanged<String> onSubmitted;
  final void Function(String secret, bool remember)? onSubmittedWithRemember;
  final String? initialSecret;

  const PinDialog({
    super.key,
    required this.deviceName,
    required this.onSubmitted,
    this.onSubmittedWithRemember,
    this.initialSecret,
  });

  @override
  State<PinDialog> createState() => _PinDialogState();
}

class _PinDialogState extends State<PinDialog> {
  late final TextEditingController _inputController;
  final FocusNode _focusNode = FocusNode();
  bool _isUnattendedMode = false;
  bool _obscurePassword = true;
  bool _rememberPassword = false;
  bool _isValid = false;

  @override
  void initState() {
    super.initState();
    _inputController = TextEditingController(text: widget.initialSecret ?? '');
    if (widget.initialSecret != null && widget.initialSecret!.isNotEmpty) {
      if (!RegExp(r'^\d{6}$').hasMatch(widget.initialSecret!)) {
        _isUnattendedMode = true;
        _rememberPassword = true;
      }
    }
    _inputController.addListener(_handleTextChange);
    _validate();
  }

  void _validate() {
    final text = _inputController.text;
    bool valid;
    if (_isUnattendedMode) {
      valid = text.trim().isNotEmpty;
    } else {
      valid = RegExp(r'^\d{6}$').hasMatch(text);
    }
    if (_isValid != valid) {
      setState(() {
        _isValid = valid;
      });
    }
  }

  void _handleTextChange() {
    _validate();
  }

  @override
  void dispose() {
    _inputController.removeListener(_handleTextChange);
    _inputController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _submit() {
    if (_isValid) {
      final secret = _inputController.text.trim();
      Navigator.of(context).pop();
      widget.onSubmitted(secret);
      widget.onSubmittedWithRemember?.call(secret, _rememberPassword);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('Connect to ${widget.deviceName}'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Mode Selector
          SegmentedButton<bool>(
            key: const Key('auth_mode_selector'),
            segments: const [
              ButtonSegment<bool>(
                value: false,
                label: Text('One-Time PIN'),
                icon: Icon(Icons.dialpad, size: 16),
              ),
              ButtonSegment<bool>(
                value: true,
                label: Text('Unattended'),
                icon: Icon(Icons.vpn_key, size: 16),
              ),
            ],
            selected: {_isUnattendedMode},
            onSelectionChanged: (selected) {
              setState(() {
                _isUnattendedMode = selected.first;
                _validate();
              });
            },
          ),
          const SizedBox(height: 16),
          Text(
            _isUnattendedMode
                ? 'Enter the permanent Unattended Access password configured on the remote host:'
                : 'Enter the 6-digit dynamic PIN displayed on the remote device:',
            style: const TextStyle(fontSize: 13, color: Colors.grey),
          ),
          const SizedBox(height: 12),
          if (!_isUnattendedMode)
            TextField(
              key: const Key('pin_input_field'),
              controller: _inputController,
              focusNode: _focusNode,
              autofocus: true,
              keyboardType: TextInputType.number,
              maxLength: 6,
              textAlign: TextAlign.center,
              style: const TextStyle(
                letterSpacing: 8,
                fontSize: 24,
                fontWeight: FontWeight.bold,
              ),
              decoration: const InputDecoration(
                counterText: '',
                hintText: '000000',
                border: OutlineInputBorder(),
              ),
              onSubmitted: (_) => _submit(),
            )
          else
            TextField(
              key: const Key('unattended_password_field'),
              controller: _inputController,
              focusNode: _focusNode,
              autofocus: true,
              obscureText: _obscurePassword,
              keyboardType: TextInputType.visiblePassword,
              decoration: InputDecoration(
                hintText: 'Enter permanent password',
                border: const OutlineInputBorder(),
                prefixIcon: const Icon(Icons.password),
                suffixIcon: IconButton(
                  key: const Key('toggle_password_visibility_button'),
                  icon: Icon(
                    _obscurePassword ? Icons.visibility : Icons.visibility_off,
                  ),
                  onPressed: () {
                    setState(() {
                      _obscurePassword = !_obscurePassword;
                    });
                  },
                ),
              ),
              onSubmitted: (_) => _submit(),
            ),
          if (_isUnattendedMode) ...[
            const SizedBox(height: 8),
            Row(
              children: [
                Checkbox(
                  key: const Key('remember_password_checkbox'),
                  value: _rememberPassword,
                  onChanged: (val) {
                    setState(() {
                      _rememberPassword = val ?? false;
                    });
                  },
                ),
                const Expanded(
                  child: Text(
                    'Remember password for this device',
                    style: TextStyle(fontSize: 13),
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
      actions: [
        TextButton(
          key: const Key('pin_cancel_button'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          key: const Key('pin_submit_button'),
          onPressed: _isValid ? _submit : null,
          child: const Text('Connect'),
        ),
      ],
    );
  }
}
