package com.vrvdesk.app

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.graphics.Path
import android.os.Build
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import androidx.annotation.RequiresApi

class InputAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "InputAccessibility"
        @Volatile
        var sharedInstance: InputAccessibilityService? = null
            private set

        val isServiceRunning: Boolean
            get() = sharedInstance != null
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        sharedInstance = this
        Log.i(TAG, "InputAccessibilityService connected and ready")
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // No-op: we only inject input gestures and actions
    }

    override fun onInterrupt() {
        Log.w(TAG, "InputAccessibilityService interrupted")
    }

    override fun onDestroy() {
        if (sharedInstance == this) {
            sharedInstance = null
        }
        Log.i(TAG, "InputAccessibilityService destroyed")
        super.onDestroy()
    }

    /**
     * Injects a tap gesture at the specified screen coordinates (x, y).
     * Duration is 40ms.
     */
    fun tap(x: Float, y: Float, callback: ((Boolean) -> Unit)? = null): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.N) {
            Log.e(TAG, "Gestures require Android N (API 24) or higher")
            callback?.invoke(false)
            return false
        }

        val path = Path().apply {
            moveTo(x, y)
            lineTo(x, y)
        }

        val stroke = GestureDescription.StrokeDescription(path, 0, 40)
        val gesture = GestureDescription.Builder()
            .addStroke(stroke)
            .build()

        return dispatchGesture(gesture, object : GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription?) {
                super.onCompleted(gestureDescription)
                Log.d(TAG, "Tap completed at ($x, $y)")
                callback?.invoke(true)
            }

            override fun onCancelled(gestureDescription: GestureDescription?) {
                super.onCancelled(gestureDescription)
                Log.w(TAG, "Tap cancelled at ($x, $y)")
                callback?.invoke(false)
            }
        }, null)
    }

    /**
     * Injects a swipe gesture from (x1, y1) to (x2, y2) over the given duration (ms).
     */
    fun swipe(
        x1: Float,
        y1: Float,
        x2: Float,
        y2: Float,
        duration: Long = 300L,
        callback: ((Boolean) -> Unit)? = null
    ): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.N) {
            Log.e(TAG, "Gestures require Android N (API 24) or higher")
            callback?.invoke(false)
            return false
        }

        val path = Path().apply {
            moveTo(x1, y1)
            lineTo(x2, y2)
        }

        val effectiveDuration = if (duration <= 0) 300L else duration
        val stroke = GestureDescription.StrokeDescription(path, 0, effectiveDuration)
        val gesture = GestureDescription.Builder()
            .addStroke(stroke)
            .build()

        return dispatchGesture(gesture, object : GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription?) {
                super.onCompleted(gestureDescription)
                Log.d(TAG, "Swipe completed from ($x1, $y1) to ($x2, $y2) in ${effectiveDuration}ms")
                callback?.invoke(true)
            }

            override fun onCancelled(gestureDescription: GestureDescription?) {
                super.onCancelled(gestureDescription)
                Log.w(TAG, "Swipe cancelled from ($x1, $y1) to ($x2, $y2)")
                callback?.invoke(false)
            }
        }, null)
    }

    /**
     * Performs a global navigation action (back, home, recents, notifications, quick_settings, etc.).
     */
    fun performGlobal(actionName: String): Boolean {
        val action = when (actionName.lowercase()) {
            "back" -> GLOBAL_ACTION_BACK
            "home" -> GLOBAL_ACTION_HOME
            "recents" -> GLOBAL_ACTION_RECENTS
            "notifications" -> GLOBAL_ACTION_NOTIFICATIONS
            "quick_settings" -> GLOBAL_ACTION_QUICK_SETTINGS
            "power_dialog" -> GLOBAL_ACTION_POWER_DIALOG
            "lock_screen" -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) GLOBAL_ACTION_LOCK_SCREEN else -1
            "take_screenshot" -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) GLOBAL_ACTION_TAKE_SCREENSHOT else -1
            else -> {
                Log.w(TAG, "Unknown global action: $actionName")
                return false
            }
        }

        if (action == -1) {
            Log.w(TAG, "Action $actionName not supported on this Android version")
            return false
        }

        val result = performGlobalAction(action)
        Log.d(TAG, "performGlobalAction($actionName) result: $result")
        return result
    }
}
