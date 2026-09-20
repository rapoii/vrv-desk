package com.vrvdesk.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioPlaybackCaptureConfiguration
import android.media.AudioRecord
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.IBinder
import android.util.Log
import android.view.Surface
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicBoolean

class MediaProjectionService : Service() {

    companion object {
        private const val TAG = "MediaProjectionService"
        const val CHANNEL_ID = "vrv_desk_capture"
        private const val NOTIFICATION_ID = 20261

        const val ACTION_START = "com.vrvdesk.app.action.START_CAPTURE"
        const val ACTION_STOP = "com.vrvdesk.app.action.STOP_CAPTURE"

        const val EXTRA_RESULT_CODE = "result_code"
        const val EXTRA_DATA = "data"
        const val EXTRA_WIDTH = "width"
        const val EXTRA_HEIGHT = "height"
        const val EXTRA_BITRATE = "bitrate"
        const val EXTRA_FPS = "fps"
        const val EXTRA_ENABLE_AUDIO = "enable_audio"

        private val VH24_MAGIC = byteArrayOf(0x56, 0x48, 0x32, 0x34) // "VH24"
        private val VAUD_MAGIC = byteArrayOf(0x56, 0x41, 0x55, 0x44) // "VAUD"
        private const val AUDIO_FORMAT_PCM_S16LE: Byte = 0x01
        private const val AUDIO_CHANNELS_STEREO: Byte = 0x02
        private const val AUDIO_SAMPLE_RATE_48K = 48000

        // Callback for streaming encoded VH24 video and VAUD audio packets
        var frameListener: ((ByteArray) -> Unit)? = null
        var isRunning: Boolean = false
            private set
    }

    private var mediaProjection: MediaProjection? = null
    private var virtualDisplay: VirtualDisplay? = null
    private var mediaCodec: MediaCodec? = null
    private var inputSurface: Surface? = null

    private var encoderThread: Thread? = null
    private val isCapturing = AtomicBoolean(false)
    private var sequenceNumber: Int = 0

    private var audioRecord: AudioRecord? = null
    private var audioCaptureThread: Thread? = null
    private val isAudioCapturing = AtomicBoolean(false)

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action ?: return START_NOT_STICKY

        when (action) {
            ACTION_START -> {
                val resultCode = intent.getIntExtra(EXTRA_RESULT_CODE, 0)
                val resultData = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(EXTRA_DATA, Intent::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(EXTRA_DATA)
                }

                val width = intent.getIntExtra(EXTRA_WIDTH, 1280)
                val height = intent.getIntExtra(EXTRA_HEIGHT, 720)
                val bitrate = intent.getIntExtra(EXTRA_BITRATE, 2_500_000)
                val fps = intent.getIntExtra(EXTRA_FPS, 30)
                val enableAudio = intent.getBooleanExtra(EXTRA_ENABLE_AUDIO, true)

                if (resultData != null && resultCode != 0) {
                    startForeground(NOTIFICATION_ID, buildNotification())
                    startScreenCapture(resultCode, resultData, width, height, bitrate, fps, enableAudio)
                } else {
                    Log.e(TAG, "Missing resultCode or intentData for startCapture")
                    stopSelf()
                }
            }
            ACTION_STOP -> {
                stopScreenCapture()
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                    stopForeground(STOP_FOREGROUND_REMOVE)
                } else {
                    @Suppress("DEPRECATION")
                    stopForeground(true)
                }
                stopSelf()
            }
        }

        return START_NOT_STICKY
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "VrV Desk Screen Capture",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Streaming active Android screen to connected VrV Desk clients"
                setShowBadge(false)
            }
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        val title = "VrV Desk Screen Streaming"
        val content = "Active screen capture in progress"

        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
                .setContentTitle(title)
                .setContentText(content)
                .setSmallIcon(android.R.drawable.ic_menu_camera)
                .setOngoing(true)
                .build()
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
                .setContentTitle(title)
                .setContentText(content)
                .setSmallIcon(android.R.drawable.ic_menu_camera)
                .setOngoing(true)
                .build()
        }
    }

    @Synchronized
    private fun startScreenCapture(
        resultCode: Int,
        resultData: Intent,
        width: Int,
        height: Int,
        bitrate: Int,
        fps: Int,
        enableAudio: Boolean = true
    ) {
        if (isCapturing.get()) {
            Log.w(TAG, "Screen capture is already active")
            return
        }

        try {
            sequenceNumber = 0

            // 1. Setup MediaCodec H.264 Encoder
            val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height).apply {
                setInteger(
                    MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface
                )
                setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
                setInteger(MediaFormat.KEY_FRAME_RATE, fps)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1) // 1 second keyframe interval
                // Set low latency encoding if supported
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    setInteger(MediaFormat.KEY_LATENCY, 0)
                }
                try {
                    setInteger(
                        MediaFormat.KEY_BITRATE_MODE,
                        MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR
                    )
                } catch (ignored: Exception) {}
            }

            val codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            inputSurface = codec.createInputSurface()
            codec.start()
            mediaCodec = codec

            // 2. Setup MediaProjection & VirtualDisplay
            val projectionManager = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
            val projection = projectionManager.getMediaProjection(resultCode, resultData)
            if (projection == null) {
                Log.e(TAG, "Failed to obtain MediaProjection token from intent")
                stopScreenCapture()
                return
            }
            mediaProjection = projection

            val densityDpi = resources.displayMetrics.densityDpi
            virtualDisplay = projection.createVirtualDisplay(
                "VrVDeskCaptureDisplay",
                width,
                height,
                densityDpi,
                DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                inputSurface,
                null,
                null
            )

            isCapturing.set(true)
            isRunning = true

            // 3. Start Encoder Drain Loop
            encoderThread = Thread({ drainEncoder() }, "VrVDesk-EncoderDrain").apply {
                isDaemon = true
                start()
            }

            // 4. Start Internal Audio Capture (Android 10+)
            if (enableAudio) {
                startAudioCapture(projection)
            }

            Log.i(TAG, "Screen capture started successfully (${width}x${height}, ${bitrate}bps, ${fps}fps, audio=$enableAudio)")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start screen capture: ${e.message}", e)
            stopScreenCapture()
        }
    }

    private fun startAudioCapture(projection: MediaProjection) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            Log.w(TAG, "AudioPlaybackCapture is only supported on Android 10 (API 29)+")
            return
        }

        try {
            val config = AudioPlaybackCaptureConfiguration.Builder(projection)
                .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                .addMatchingUsage(AudioAttributes.USAGE_GAME)
                .addMatchingUsage(AudioAttributes.USAGE_UNKNOWN)
                .build()

            val audioFormat = AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(AUDIO_SAMPLE_RATE_48K)
                .setChannelMask(AudioFormat.CHANNEL_IN_STEREO)
                .build()

            val minBufferSize = AudioRecord.getMinBufferSize(
                AUDIO_SAMPLE_RATE_48K,
                AudioFormat.CHANNEL_IN_STEREO,
                AudioFormat.ENCODING_PCM_16BIT
            )

            // 20ms chunk at 48kHz stereo 16-bit = 48000 * 2 channels * 2 bytes * 0.02s = 3840 bytes
            val chunkSize = 3840
            val bufferSize = minBufferSize.coerceAtLeast(chunkSize * 4)

            val record = AudioRecord.Builder()
                .setAudioPlaybackCaptureConfig(config)
                .setAudioFormat(audioFormat)
                .setBufferSizeInBytes(bufferSize)
                .build()

            if (record.state != AudioRecord.STATE_INITIALIZED) {
                Log.e(TAG, "AudioRecord failed to initialize for internal playback capture")
                record.release()
                return
            }

            record.startRecording()
            audioRecord = record
            isAudioCapturing.set(true)

            audioCaptureThread = Thread({ drainAudio(record, chunkSize) }, "VrVDesk-AudioDrain").apply {
                isDaemon = true
                start()
            }

            Log.i(TAG, "Internal audio capture started successfully (48kHz Stereo PCM)")
        } catch (e: SecurityException) {
            Log.e(TAG, "SecurityException starting internal audio capture: ${e.message}")
        } catch (e: Exception) {
            Log.e(TAG, "Error starting internal audio capture: ${e.message}", e)
        }
    }

    private fun drainAudio(record: AudioRecord, chunkSize: Int) {
        val buffer = ByteArray(chunkSize)
        while (isAudioCapturing.get() && isCapturing.get()) {
            try {
                val readBytes = record.read(buffer, 0, buffer.size)
                if (readBytes > 0) {
                    val chunk = if (readBytes == buffer.size) buffer else buffer.copyOf(readBytes)
                    val vaudPacket = packageVaudFrame(chunk)
                    frameListener?.invoke(vaudPacket)
                } else if (readBytes < 0) {
                    Log.w(TAG, "AudioRecord read error code: $readBytes")
                    break
                }
            } catch (e: Exception) {
                if (isAudioCapturing.get()) {
                    Log.e(TAG, "Error draining AudioRecord: ${e.message}")
                }
                break
            }
        }
    }

    private fun drainEncoder() {
        val codec = mediaCodec ?: return
        val bufferInfo = MediaCodec.BufferInfo()
        val timeoutUs = 10_000L // 10ms poll timeout

        var configHeader: ByteArray? = null

        while (isCapturing.get()) {
            try {
                val outputBufferIndex = codec.dequeueOutputBuffer(bufferInfo, timeoutUs)
                if (outputBufferIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    val newFormat = codec.outputFormat
                    Log.i(TAG, "Encoder output format changed: $newFormat")
                    // Extract SPS/PPS if present in csd-0 / csd-1
                    val sps = newFormat.getByteBuffer("csd-0")
                    val pps = newFormat.getByteBuffer("csd-1")
                    if (sps != null && pps != null) {
                        val header = ByteArray(sps.remaining() + pps.remaining())
                        sps.get(header, 0, sps.remaining())
                        pps.get(header, sps.remaining(), pps.remaining())
                        configHeader = header
                    }
                } else if (outputBufferIndex >= 0) {
                    val outputBuffer = codec.getOutputBuffer(outputBufferIndex)
                    if (outputBuffer != null && bufferInfo.size > 0) {
                        outputBuffer.position(bufferInfo.offset)
                        outputBuffer.limit(bufferInfo.offset + bufferInfo.size)

                        val chunk = ByteArray(bufferInfo.size)
                        outputBuffer.get(chunk)

                        val isConfig = (bufferInfo.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0
                        val isKeyframe = (bufferInfo.flags and MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0

                        if (isConfig) {
                            configHeader = chunk
                        } else {
                            val nalPayload: ByteArray = if (isKeyframe && configHeader != null) {
                                // Prepend SPS/PPS if not already present in this IDR frame chunk
                                if (!hasSpsPps(chunk)) {
                                    val combined = ByteArray(configHeader.size + chunk.size)
                                    System.arraycopy(configHeader, 0, combined, 0, configHeader.size)
                                    System.arraycopy(chunk, 0, combined, configHeader.size, chunk.size)
                                    combined
                                } else {
                                    chunk
                                }
                            } else {
                                chunk
                            }

                            val vh24Packet = packageVh24Frame(nalPayload, isKeyframe, sequenceNumber++)
                            frameListener?.invoke(vh24Packet)
                        }
                    }

                    codec.releaseOutputBuffer(outputBufferIndex, false)

                    if ((bufferInfo.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0) {
                        break
                    }
                }
            } catch (e: Exception) {
                if (isCapturing.get()) {
                    Log.e(TAG, "Error draining encoder: ${e.message}", e)
                }
                break
            }
        }
    }

    private fun hasSpsPps(data: ByteArray): Boolean {
        var i = 0
        while (i + 4 < data.size) {
            if (data[i].toInt() == 0 && data[i + 1].toInt() == 0) {
                if (data[i + 2].toInt() == 0 && data[i + 3].toInt() == 1) {
                    val nalType = (data[i + 4].toInt() and 0x1F)
                    if (nalType == 7 || nalType == 8) return true
                    i += 4
                    continue
                } else if (data[i + 2].toInt() == 1) {
                    val nalType = (data[i + 3].toInt() and 0x1F)
                    if (nalType == 7 || nalType == 8) return true
                    i += 3
                    continue
                }
            }
            i++
        }
        return false
    }

    /**
     * Formats H.264 NALUs with 12-byte VH24 header:
     * [0..4]   b"VH24"
     * [4..8]   length of NAL payload (u32 big endian)
     * [8]      flags (0x01 if keyframe, 0x00 otherwise)
     * [9..12]  sequence number (u32 big endian)
     * [12..]   NAL payload
     */
    private fun packageVh24Frame(nalPayload: ByteArray, isKeyframe: Boolean, seq: Int): ByteArray {
        val totalLength = 12 + nalPayload.size
        val packet = ByteArray(totalLength)

        // [0..4] b"VH24"
        System.arraycopy(VH24_MAGIC, 0, packet, 0, 4)

        // [4..8] length (u32 be)
        val len = nalPayload.size
        packet[4] = ((len ushr 24) and 0xFF).toByte()
        packet[5] = ((len ushr 16) and 0xFF).toByte()
        packet[6] = ((len ushr 8) and 0xFF).toByte()
        packet[7] = (len and 0xFF).toByte()

        // [8] flags (0x01 if keyframe)
        packet[8] = if (isKeyframe) 0x01.toByte() else 0x00.toByte()

        // [9..12] sequence (u32 be)
        packet[9] = ((seq ushr 24) and 0xFF).toByte()
        packet[10] = ((seq ushr 16) and 0xFF).toByte()
        packet[11] = ((seq ushr 8) and 0xFF).toByte()
        packet[12] = (seq and 0xFF).toByte()

        // [12..] NAL payload
        System.arraycopy(nalPayload, 0, packet, 12, nalPayload.size)

        return packet
    }

    /**
     * Packages raw PCM audio buffer into standard 8-byte VAUD packet:
     * [0..4]   b"VAUD" (ASCII 0x56, 0x41, 0x55, 0x44)
     * [4]      format (0x01 = PCM S16LE)
     * [5]      channels (0x02 = Stereo)
     * [6..8]   sample rate as 16-bit little-endian integer (48000 -> 0x80, 0xBB)
     * [8..]    raw PCM payload
     */
    private fun packageVaudFrame(pcmPayload: ByteArray): ByteArray {
        val totalLength = 8 + pcmPayload.size
        val packet = ByteArray(totalLength)

        // [0..4] b"VAUD"
        System.arraycopy(VAUD_MAGIC, 0, packet, 0, 4)

        // [4] format: 0x01
        packet[4] = AUDIO_FORMAT_PCM_S16LE

        // [5] channels: 2
        packet[5] = AUDIO_CHANNELS_STEREO

        // [6..8] sample rate: 48000 little-endian
        packet[6] = (AUDIO_SAMPLE_RATE_48K and 0xFF).toByte()
        packet[7] = ((AUDIO_SAMPLE_RATE_48K ushr 8) and 0xFF).toByte()

        // [8..] PCM payload
        System.arraycopy(pcmPayload, 0, packet, 8, pcmPayload.size)

        return packet
    }

    private fun stopAudioCapture() {
        if (!isAudioCapturing.getAndSet(false)) return
        try {
            audioCaptureThread?.interrupt()
            audioCaptureThread = null

            audioRecord?.apply {
                if (state == AudioRecord.STATE_INITIALIZED) {
                    try {
                        stop()
                    } catch (ignored: Exception) {}
                }
                release()
            }
            audioRecord = null
            Log.i(TAG, "Internal audio capture stopped successfully")
        } catch (e: Exception) {
            Log.w(TAG, "Error stopping audio capture: ${e.message}")
        }
    }

    @Synchronized
    private fun stopScreenCapture() {
        if (!isCapturing.getAndSet(false)) {
            return
        }
        isRunning = false

        // Stop internal audio capture
        stopAudioCapture()

        try {
            encoderThread?.interrupt()
            encoderThread = null

            virtualDisplay?.release()
            virtualDisplay = null

            mediaProjection?.stop()
            mediaProjection = null

            inputSurface?.release()
            inputSurface = null

            mediaCodec?.stop()
            mediaCodec?.release()
            mediaCodec = null

            Log.i(TAG, "Screen capture stopped successfully")
        } catch (e: Exception) {
            Log.w(TAG, "Error stopping screen capture: ${e.message}")
        }
    }

    override fun onDestroy() {
        stopScreenCapture()
        frameListener = null
        super.onDestroy()
    }
}
