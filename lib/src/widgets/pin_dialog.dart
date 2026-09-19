import 'package:flutter/material.dart';

class PinDialog extends StatefulWidget {
  final String deviceName;
  final ValueChanged<String> onSubmitted;

  const PinDialog({
    super.key,
    required this.deviceName,
    required this.onSubmitted,
  });

  @override
  State<PinDialog> createState() => _PinDialogState();
}

class _PinDialogState extends State<PinDialog> {
  final TextEditingController _pinController = TextEditingController();
  final FocusNode _focusNode = FocusNode();
  bool _isValid = false;

  @override
  void initState() {
    super.initState();
    _pinController.addListener(_handleTextChange);
  }

  void _handleTextChange() {
    final text = _pinController.text;
    final isSixDigits = RegExp(r'^\d{6}$').hasMatch(text);
    if (_isValid != isSixDigits) {
      setState(() {
        _isValid = isSixDigits;
      });
    }
  }

  @override
  void dispose() {
    _pinController.removeListener(_handleTextChange);
    _pinController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _submit() {
    if (_isValid) {
      final pin = _pinController.text;
      Navigator.of(context).pop();
      widget.onSubmitted(pin);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('Connect to ${widget.deviceName}'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('Enter the 6-digit PIN displayed on the remote device:'),
          const SizedBox(height: 16),
          TextField(
            key: const Key('pin_input_field'),
            controller: _pinController,
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
          ),
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
