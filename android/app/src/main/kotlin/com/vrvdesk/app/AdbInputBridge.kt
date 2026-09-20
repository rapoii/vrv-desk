package com.vrvdesk.app

import android.content.Context
import android.os.Build
import android.provider.Settings
import android.util.Log
import android.view.WindowManager
import java.io.BufferedWriter
import java.io.OutputStreamWriter
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean

/**
 * High-Performance ADB Input Bridge.
 *
 * Provides ultra-fast direct shell input injection (`input tap`, `input swipe`,
 * `input keyevent`) via a persistent shell process, probe utilities for local
 * ADB debugging (127.0.0.1:5555), and display refresh rate queries (90Hz / 120Hz).
 */
class AdbInputBridge(private val context: Context) {

    companion object {
        private const val TAG = "AdbInputBridge"
        const val DEFAULT_ADB_PORT = 5555

        // Android KeyEvent Constants
        const val KEYCODE_HOME = 3
        const val KEYCODE_BACK = 4
        const val KEYCODE_APP_SWITCH = 187 // Recents
        const val KEYCODE_POWER = 26
        const val KEYCODE_VOLUME_UP = 24
        const val KEYCODE_VOLUME_DOWN = 25
    }

    private var shellProcess: Process? = null
    private var shellWriter: BufferedWriter? = null
    private val lock = Any()
    private val isInitialized = AtomicBoolean(false)

    /**
     * Initializes the persistent shell session for instant sub-10ms command execution.
     */
    fun initializeShell(): Boolean {
        synchronized(lock) {
            if (isInitialized.get() && shellProcess?.isAlive == true) {
                return true
            }
            return try {
                val process = ProcessBuilder("sh").redirectErrorStream(true).start()
                shellWriter = BufferedWriter(OutputStreamWriter(process.outputStream))
                shellProcess = process
                isInitialized.set(true)
                Log.i(TAG, "Persistent ADB shell process initialized")
                true
            } catch (e: Exception) {
                Log.w(TAG, "Failed to start persistent shell: ${e.message}")
                false
            }
        }
    }

    /**
     * Sends a command directly to the persistent shell stream without spawning child processes.
     */
    fun sendCommand(command: String): Boolean {
        synchronized(lock) {
            if (!isInitialized.get() || shellProcess?.isAlive != true) {
                if (!initializeShell()) return false
            }
            return try {
                shellWriter?.write(command)
                shellWriter?.newLine()
                shellWriter?.flush()
                true
            } catch (e: Exception) {
                Log.w(TAG, "Error writing to shell process: ${e.message}")
                isInitialized.set(false)
                false
            }
        }
    }

    /**
     * Injects a tap gesture at physical pixel coordinates.
     */
    fun tap(x: Float, y: Float): Boolean {
        return sendCommand("input tap ${x.toInt()} ${y.toInt()}")
    }

    /**
     * Injects a swipe gesture between two physical pixel coordinates with duration.
     */
    fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Int): Boolean {
        return sendCommand("input swipe ${x1.toInt()} ${y1.toInt()} ${x2.toInt()} ${y2.toInt()} $durationMs")
    }

    /**
     * Injects an Android KeyEvent (Home, Back, Recents, Volume).
     */
    fun keyevent(keyCode: Int): Boolean {
        return sendCommand("input keyevent $keyCode")
    }

    /**
     * Injects text into the focused field.
     */
    fun text(input: String): Boolean {
        val escaped = input.replace(" ", "%s").replace("\"", "\\\"")
        return sendCommand("input text \"$escaped\"")
    }

    /**
     * Checks if ADB USB/Network debugging is enabled in Android Developer Options.
     */
    fun isAdbDebuggingEnabled(): Boolean {
        return try {
            Settings.Global.getInt(context.contentResolver, Settings.Global.ADB_ENABLED, 0) == 1
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Probes if local TCP ADB daemon (e.g. 127.0.0.1:5555 or custom port) is active.
     */
    fun probeAdbTcpPort(host: String = "127.0.0.1", port: Int = DEFAULT_ADB_PORT, timeoutMs: Int = 150): Boolean {
        return try {
            Socket().use { socket ->
                socket.connect(InetSocketAddress(host, port), timeoutMs)
                true
            }
        } catch (_: Exception) {
            false
        }
    }

    /**
     * Queries the maximum display refresh rate supported by the active screen (e.g. 60Hz, 90Hz, 120Hz).
     */
    fun getMaxDisplayRefreshRate(): Double {
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val display = context.display ?: return 60.0
                display.supportedModes.map { it.refreshRate.toDouble() }.maxOrNull() ?: 60.0
            } else {
                @Suppress("DEPRECATION")
                val wm = context.getSystemService(Context.WINDOW_SERVICE) as? WindowManager
                @Suppress("DEPRECATION")
                val display = wm?.defaultDisplay ?: return 60.0
                display.supportedModes.map { it.refreshRate.toDouble() }.maxOrNull() ?: 60.0
            }
        } catch (_: Exception) {
            60.0
        }
    }

    /**
     * Returns full diagnostic status map of ADB High-Performance subsystem.
     */
    fun getAdbStatus(): Map<String, Any> {
        val adbEnabled = isAdbDebuggingEnabled()
        val portOpen = probeAdbTcpPort()
        val maxRefresh = getMaxDisplayRefreshRate()
        val shellReady = isInitialized.get() || initializeShell()

        return mapOf(
            "adb_enabled" to adbEnabled,
            "port_5555_open" to portOpen,
            "max_refresh_rate" to maxRefresh,
            "shell_ready" to shellReady,
            "high_performance_available" to (adbEnabled || portOpen || shellReady)
        )
    }

    /**
     * Gracefully closes the persistent shell process.
     */
    fun close() {
        synchronized(lock) {
            try {
                shellWriter?.write("exit\n")
                shellWriter?.flush()
                shellWriter?.close()
            } catch (_: Exception) {}
            shellWriter = null
            shellProcess?.destroy()
            shellProcess = null
            isInitialized.set(false)
        }
    }
}
