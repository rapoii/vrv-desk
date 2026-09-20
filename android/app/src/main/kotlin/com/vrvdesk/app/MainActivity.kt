package com.vrvdesk.app

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.Build
import android.util.Log
import android.view.Surface
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

class MainActivity : FlutterActivity() {
    private val channelName = "com.vrv.desk/audio"
    private val videoChannelName = "com.vrv.desk/video"
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

    companion object {
        private const val TAG = "AudioTrackNative"
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

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
        stopAudioTrack()
        disposeVideo()
        audioExecutor.shutdown()
        videoExecutor.shutdown()
        super.onDestroy()
    }
}
