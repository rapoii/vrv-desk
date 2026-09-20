package com.vrvdesk.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Surface
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

class MainActivity : FlutterActivity() {
    private val channelName = "com.vrv.desk/audio"
    private val videoChannelName = "com.vrv.desk/video"
    private val captureChannelName = "com.vrv.desk/capture"
    private val captureStreamChannelName = "com.vrv.desk/capture_stream"
    private val accessibilityChannelName = "com.vrv.desk/accessibility"

    private var audioTrack: AudioTrack? = null
    private var opusDecoder: OpusAudioDecoder? = null
    private var videoTextureEntry: TextureRegistry.SurfaceTextureEntry? = null
    private var videoSurface: Surface? = null
    private var h264Decoder: H264VideoDecoder? = null
    private var isMuted: Boolean = false
    private var currentSampleRate: Int = 0
    private var currentChannels: Int = 0
    private val audioExecutor: ExecutorService = Executors.newSingleThreadExecutor()
    private val videoExecutor: ExecutorService = Executors.newSingleThreadExecutor()
    private var totalChunksWritten: Long = 0
    private var nonZeroChunksWritten: Long = 0
    private var peakAmplitude: Int = 0
    private var totalBytesWritten: Long = 0
    private var opusFramesDecoded: Long = 0

    // Capture & Streaming state
    private var captureStreamSink: EventChannel.EventSink? = null
    private var pendingCaptureResult: MethodChannel.Result? = null
    private val mainHandler = Handler(Looper.getMainLooper())

    companion object {
        private const val TAG = "MainActivity"
        private const val MEDIA_PROJECTION_REQUEST_CODE = 1001
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        // Audio MethodChannel
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName).setMethodCallHandler { call, result ->
            when (call.method) {
                "init" -> {
                    val sampleRate = call.argument<Int>("sampleRate") ?: 48000
                    val channels = call.argument<Int>("channels") ?: 2
                    val success = initAudioTrack(sampleRate, channels)
                    result.success(success)
                }
                "write" -> {
                    val data = call.argument<ByteArray>("data")
                    val format = call.argument<Int>("format") ?: 1
                    if (data != null) {
                        audioExecutor.execute {
                            if (format == 2) {
                                opusFramesDecoded++
                                opusDecoder?.decode(data)
                            } else {
                                writeAudioData(data)
                            }
                        }
                        result.success(true)
                    } else {
                        result.error("INVALID_ARGUMENT", "Audio data byte array is null", null)
                    }
                }
                "setMuted" -> {
                    val muted = call.argument<Boolean>("muted") ?: false
                    setMutedState(muted)
                    result.success(true)
                }
                "stop" -> {
                    stopAudioTrack()
                    result.success(true)
                }
                else -> {
                    result.notImplemented()
                }
            }
        }

        // Video MethodChannel
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, videoChannelName).setMethodCallHandler { call, result ->
            when (call.method) {
                "init" -> {
                    val width = call.argument<Int>("width") ?: 1280
                    val height = call.argument<Int>("height") ?: 720
                    try {
                        disposeVideo()
                        val textureEntry = flutterEngine.renderer.createSurfaceTexture()
                        val surfaceTexture = textureEntry.surfaceTexture()
                        surfaceTexture.setDefaultBufferSize(width, height)
                        val surface = Surface(surfaceTexture)

                        val decoder = H264VideoDecoder(surface)
                        val ok = decoder.init(width, height)
                        if (ok) {
                            videoTextureEntry = textureEntry
                            videoSurface = surface
                            h264Decoder = decoder
                            result.success(textureEntry.id())
                        } else {
                            surface.release()
                            textureEntry.release()
                            result.error("INIT_FAILED", "Failed to init MediaCodec H.264 decoder", null)
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Error setting up video texture: ${e.message}", e)
                        result.error("EXCEPTION", e.message, null)
                    }
                }
                "write" -> {
                    val data = call.argument<ByteArray>("data")
                    if (data != null) {
                        videoExecutor.execute {
                            h264Decoder?.decode(data)
                        }
                        result.success(true)
                    } else {
                        result.error("INVALID_ARGUMENT", "Video NAL data is null", null)
                    }
                }
                "dispose" -> {
                    disposeVideo()
                    result.success(true)
                }
                else -> {
                    result.notImplemented()
                }
            }
        }

        // Capture MethodChannel ('com.vrv.desk/capture')
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, captureChannelName).setMethodCallHandler { call, result ->
            when (call.method) {
                "requestCapturePermission" -> {
                    try {
                        val projectionManager = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
                        val intent = projectionManager.createScreenCaptureIntent()
                        pendingCaptureResult = result
                        startActivityForResult(intent, MEDIA_PROJECTION_REQUEST_CODE)
                    } catch (e: Exception) {
                        Log.e(TAG, "Error requesting MediaProjection permission: ${e.message}", e)
                        result.error("PERMISSION_ERROR", e.message, null)
                    }
                }
                "startCapture" -> {
                    val resultCode = call.argument<Int>("resultCode") ?: Activity.RESULT_OK
                    val intentData = call.argument<Intent>("intentData")
                    val width = call.argument<Int>("width") ?: 1280
                    val height = call.argument<Int>("height") ?: 720
                    val bitrate = call.argument<Int>("bitrate") ?: 2_500_000
                    val fps = call.argument<Int>("fps") ?: 30

                    startCaptureService(resultCode, intentData, width, height, bitrate, fps)
                    result.success(true)
                }
                "stopCapture" -> {
                    stopCaptureService()
                    result.success(true)
                }
                "isCapturing" -> {
                    result.success(MediaProjectionService.isRunning)
                }
                else -> {
                    result.notImplemented()
                }
            }
        }

        // Capture EventChannel ('com.vrv.desk/capture_stream')
        EventChannel(flutterEngine.dartExecutor.binaryMessenger, captureStreamChannelName).setStreamHandler(
            object : EventChannel.StreamHandler {
                override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                    captureStreamSink = events
                    MediaProjectionService.frameListener = { packet ->
                        mainHandler.post {
                            captureStreamSink?.success(packet)
                        }
                    }
                    Log.i(TAG, "Capture stream listener attached")
                }

                override fun onCancel(arguments: Any?) {
                    captureStreamSink = null
                    MediaProjectionService.frameListener = null
                    Log.i(TAG, "Capture stream listener detached")
                }
            }
        )

        // Accessibility MethodChannel ('com.vrv.desk/accessibility')
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, accessibilityChannelName).setMethodCallHandler { call, result ->
            when (call.method) {
                "isAccessibilityEnabled" -> {
                    val enabled = checkAccessibilityPermission()
                    result.success(enabled)
                }
                "openAccessibilitySettings" -> {
                    try {
                        val intent = Intent(android.provider.Settings.ACTION_ACCESSIBILITY_SETTINGS).apply {
                            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        }
                        startActivity(intent)
                        result.success(true)
                    } catch (e: Exception) {
                        Log.e(TAG, "Failed to open accessibility settings: ${e.message}", e)
                        result.error("INTENT_ERROR", e.message, null)
                    }
                }
                "tap" -> {
                    val x = (call.argument<Number>("x"))?.toFloat()
                    val y = (call.argument<Number>("y"))?.toFloat()
                    val service = InputAccessibilityService.sharedInstance
                    if (service == null) {
                        result.error("SERVICE_UNAVAILABLE", "InputAccessibilityService is not enabled or connected", null)
                    } else if (x == null || y == null) {
                        result.error("INVALID_ARGUMENT", "Coordinates x and y must not be null", null)
                    } else {
                        val dispatched = service.tap(x, y) { success ->
                            // Handled synchronously by dispatch, but callback confirms gesture execution
                        }
                        result.success(dispatched)
                    }
                }
                "swipe" -> {
                    val x1 = (call.argument<Number>("x1"))?.toFloat()
                    val y1 = (call.argument<Number>("y1"))?.toFloat()
                    val x2 = (call.argument<Number>("x2"))?.toFloat()
                    val y2 = (call.argument<Number>("y2"))?.toFloat()
                    val duration = (call.argument<Number>("duration"))?.toLong() ?: 300L
                    val service = InputAccessibilityService.sharedInstance
                    if (service == null) {
                        result.error("SERVICE_UNAVAILABLE", "InputAccessibilityService is not enabled or connected", null)
                    } else if (x1 == null || y1 == null || x2 == null || y2 == null) {
                        result.error("INVALID_ARGUMENT", "Coordinates x1, y1, x2, y2 must not be null", null)
                    } else {
                        val dispatched = service.swipe(x1, y1, x2, y2, duration) { success ->
                            // Handled synchronously by dispatch
                        }
                        result.success(dispatched)
                    }
                }
                "globalAction" -> {
                    val action = call.argument<String>("action") ?: call.argument<String>("actionName")
                    val service = InputAccessibilityService.sharedInstance
                    if (service == null) {
                        result.error("SERVICE_UNAVAILABLE", "InputAccessibilityService is not enabled or connected", null)
                    } else if (action == null) {
                        result.error("INVALID_ARGUMENT", "Action name must not be null", null)
                    } else {
                        val success = service.performGlobal(action)
                        result.success(success)
                    }
                }
                else -> {
                    result.notImplemented()
                }
            }
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == MEDIA_PROJECTION_REQUEST_CODE) {
            val pending = pendingCaptureResult
            pendingCaptureResult = null

            if (resultCode == Activity.RESULT_OK && data != null) {
                lastProjectionResultCode = resultCode
                lastProjectionIntentData = data

                val resultMap = HashMap<String, Any>()
                resultMap["resultCode"] = resultCode
                resultMap["granted"] = true
                pending?.success(resultMap)
            } else {
                val resultMap = HashMap<String, Any>()
                resultMap["resultCode"] = resultCode
                resultMap["granted"] = false
                pending?.success(resultMap)
            }
        }
    }

    private var lastProjectionResultCode: Int = 0
    private var lastProjectionIntentData: Intent? = null

    private fun startCaptureService(
        resultCode: Int,
        intentData: Intent?,
        width: Int,
        height: Int,
        bitrate: Int,
        fps: Int
    ) {
        val effectiveCode = if (resultCode != 0) resultCode else lastProjectionResultCode
        val effectiveData = intentData ?: lastProjectionIntentData

        if (effectiveData == null || effectiveCode == 0) {
            Log.e(TAG, "Cannot start capture service: MediaProjection permission intent is null")
            return
        }

        val serviceIntent = Intent(this, MediaProjectionService::class.java).apply {
            action = MediaProjectionService.ACTION_START
            putExtra(MediaProjectionService.EXTRA_RESULT_CODE, effectiveCode)
            putExtra(MediaProjectionService.EXTRA_DATA, effectiveData)
            putExtra(MediaProjectionService.EXTRA_WIDTH, width)
            putExtra(MediaProjectionService.EXTRA_HEIGHT, height)
            putExtra(MediaProjectionService.EXTRA_BITRATE, bitrate)
            putExtra(MediaProjectionService.EXTRA_FPS, fps)
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(serviceIntent)
        } else {
            startService(serviceIntent)
        }
    }

    private fun stopCaptureService() {
        val serviceIntent = Intent(this, MediaProjectionService::class.java).apply {
            action = MediaProjectionService.ACTION_STOP
        }
        startService(serviceIntent)
    }

    private fun checkAccessibilityPermission(): Boolean {
        // First check if our service singleton is directly connected
        if (InputAccessibilityService.isServiceRunning) {
            return true
        }

        // Also check Android Settings secure string for accessibility enabled services
        try {
            val expectedServiceName = "${packageName}/${InputAccessibilityService::class.java.canonicalName}"
            val accessibilityEnabled = android.provider.Settings.Secure.getInt(
                contentResolver,
                android.provider.Settings.Secure.ACCESSIBILITY_ENABLED,
                0
            )
            if (accessibilityEnabled == 1) {
                val settingValue = android.provider.Settings.Secure.getString(
                    contentResolver,
                    android.provider.Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
                )
                if (settingValue != null) {
                    val splitter = android.text.TextUtils.SimpleStringSplitter(':')
                    splitter.setString(settingValue)
                    while (splitter.hasNext()) {
                        val service = splitter.next()
                        if (service.equals(expectedServiceName, ignoreCase = true) ||
                            service.contains(InputAccessibilityService::class.java.simpleName)
                        ) {
                            return true
                        }
                    }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error checking accessibility permission: ${e.message}", e)
        }
        return false
    }

    @Synchronized
    private fun disposeVideo() {
        try {
            h264Decoder?.release()
            h264Decoder = null
            videoSurface?.release()
            videoSurface = null
            videoTextureEntry?.release()
            videoTextureEntry = null
        } catch (e: Exception) {
            Log.w(TAG, "Error disposing video pipeline: ${e.message}")
        }
    }

    @Synchronized
    private fun initAudioTrack(sampleRate: Int, channels: Int): Boolean {
        try {
            if (audioTrack != null && currentSampleRate == sampleRate && currentChannels == channels) {
                if (audioTrack?.playState != AudioTrack.PLAYSTATE_PLAYING) {
                    audioTrack?.play()
                }
                return true
            }

            stopAudioTrack()

            val channelConfig = if (channels == 1) {
                AudioFormat.CHANNEL_OUT_MONO
            } else {
                AudioFormat.CHANNEL_OUT_STEREO
            }

            val minBufferSize = AudioTrack.getMinBufferSize(
                sampleRate,
                channelConfig,
                AudioFormat.ENCODING_PCM_16BIT
            )

            if (minBufferSize <= 0) {
                Log.e(TAG, "Invalid minBufferSize: $minBufferSize for rate=$sampleRate channels=$channels")
                return false
            }

            val bufferSize = minBufferSize * 2

            val attributes = AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_UNKNOWN)
                .build()

            val format = AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(sampleRate)
                .setChannelMask(channelConfig)
                .build()

            val track = AudioTrack.Builder()
                .setAudioAttributes(attributes)
                .setAudioFormat(format)
                .setBufferSizeInBytes(bufferSize)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                .build()

            if (isMuted) {
                track.setVolume(0.0f)
            } else {
                track.setVolume(1.0f)
            }

            track.play()
            audioTrack = track
            currentSampleRate = sampleRate
            currentChannels = channels

            // Initialize hardware Opus decoder
            try {
                val decoder = OpusAudioDecoder(sampleRate, channels) { pcmChunk ->
                    writeAudioData(pcmChunk)
                }
                if (decoder.init()) {
                    opusDecoder = decoder
                    Log.i(TAG, "OpusAudioDecoder attached to AudioTrack")
                } else {
                    Log.w(TAG, "OpusAudioDecoder init failed; falling back to PCM")
                    opusDecoder = null
                }
            } catch (e: Exception) {
                Log.e(TAG, "Failed creating Opus decoder", e)
                opusDecoder = null
            }

            Log.i(TAG, "AudioTrack initialized successfully (sampleRate=$sampleRate, channels=$channels, bufferSize=$bufferSize)")
            return true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize AudioTrack", e)
            audioTrack = null
            return false
        }
    }

    @Synchronized
    private fun writeAudioData(data: ByteArray) {
        val track = audioTrack ?: return
        try {
            if (track.playState != AudioTrack.PLAYSTATE_PLAYING) {
                track.play()
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                track.write(data, 0, data.size, AudioTrack.WRITE_NON_BLOCKING)
            } else {
                track.write(data, 0, data.size)
            }

            totalChunksWritten++
            totalBytesWritten += data.size

            // Check if chunk contains non-zero PCM samples (detecting real sound)
            var chunkMax = 0
            var i = 0
            while (i + 1 < data.size) {
                val low = data[i].toInt() and 0xFF
                val high = data[i + 1].toInt()
                val sample = Math.abs((high shl 8) or low)
                if (sample > chunkMax) chunkMax = sample
                i += 2
            }
            if (chunkMax > 100) {
                nonZeroChunksWritten++
            }
            if (chunkMax > peakAmplitude) {
                peakAmplitude = chunkMax
            }

            // Periodic telemetry log every 100 chunks (~2 seconds)
            if (totalChunksWritten % 100L == 0L) {
                Log.i(
                    TAG,
                    "AudioTrack stats: chunks=$totalChunksWritten, nonZero=$nonZeroChunksWritten, totalBytes=$totalBytesWritten, peakAmp=$peakAmplitude, lastChunkMax=$chunkMax, muted=$isMuted"
                )
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error writing audio chunk", e)
        }
    }

    @Synchronized
    private fun setMutedState(muted: Boolean) {
        isMuted = muted
        val track = audioTrack ?: return
        try {
            val vol = if (muted) 0.0f else 1.0f
            track.setVolume(vol)
        } catch (e: Exception) {
            Log.e(TAG, "Error setting volume/mute state", e)
        }
    }

    @Synchronized
    private fun stopAudioTrack() {
        try {
            opusDecoder?.stop()
            opusDecoder = null
            audioTrack?.let { track ->
                if (track.playState == AudioTrack.PLAYSTATE_PLAYING) {
                    track.pause()
                    track.flush()
                }
                track.release()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping AudioTrack", e)
        } finally {
            audioTrack = null
            currentSampleRate = 0
            currentChannels = 0
        }
    }

    override fun onDestroy() {
        stopCaptureService()
        stopAudioTrack()
        disposeVideo()
        audioExecutor.shutdown()
        videoExecutor.shutdown()
        super.onDestroy()
    }
}
