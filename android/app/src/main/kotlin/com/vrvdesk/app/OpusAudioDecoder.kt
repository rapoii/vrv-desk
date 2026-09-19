package com.vrvdesk.app

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import java.nio.ByteBuffer
import java.nio.ByteOrder

class OpusAudioDecoder(
    private val sampleRate: Int = 48000,
    private val channels: Int = 2,
    private val onPcmOutput: (ByteArray) -> Unit
) {
    companion object {
        private const val TAG = "OpusAudioDecoder"
    }

    private var codec: MediaCodec? = null
    private var isConfigured = false

    fun init(): Boolean {
        try {
            stop()
            val mediaCodec = MediaCodec.createDecoderByType("audio/opus")
            val format = MediaFormat.createAudioFormat("audio/opus", sampleRate, channels)

            // csd-0 header for Opus (19 bytes identification header)
            val csd0 = ByteBuffer.allocate(19).order(ByteOrder.LITTLE_ENDIAN).apply {
                put("OpusHead".toByteArray(Charsets.US_ASCII))
                put(1.toByte()) // version
                put(channels.toByte()) // channels
                putShort(0) // pre-skip
                putInt(sampleRate) // original sample rate
                putShort(0) // output gain
                put(0.toByte()) // channel mapping family (0 = mono/stereo)
            }.array()
            format.setByteBuffer("csd-0", ByteBuffer.wrap(csd0))

            // csd-1 and csd-2: pre-skip and seek pre-roll (in nanoseconds, 8-byte uint64)
            val csd1 = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(0L).array()
            val csd2 = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(80_000_000L).array()
            format.setByteBuffer("csd-1", ByteBuffer.wrap(csd1))
            format.setByteBuffer("csd-2", ByteBuffer.wrap(csd2))

            mediaCodec.configure(format, null, null, 0)
            mediaCodec.start()
            codec = mediaCodec
            isConfigured = true
            Log.i(TAG, "Opus decoder initialized successfully ($sampleRate Hz, $channels channels)")
            return true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize Opus decoder", e)
            codec = null
            isConfigured = false
            return false
        }
    }

    private var decodeCount = 0L
    private var outputCount = 0L
    private var presentationTimeUs = 0L

    fun decode(opusData: ByteArray) {
        val decoder = codec ?: return
        if (!isConfigured) return

        try {
            decodeCount++
            val inIndex = decoder.dequeueInputBuffer(5000)
            if (inIndex >= 0) {
                val inBuffer = decoder.getInputBuffer(inIndex)
                inBuffer?.clear()
                inBuffer?.put(opusData)
                presentationTimeUs += 20_000L // 20ms in microseconds
                decoder.queueInputBuffer(inIndex, 0, opusData.size, presentationTimeUs, 0)
            } else {
                Log.w(TAG, "Input buffer full or unavailable: $inIndex")
            }

            val bufferInfo = MediaCodec.BufferInfo()
            var outIndex = decoder.dequeueOutputBuffer(bufferInfo, 2000)
            while (outIndex >= 0) {
                val outBuffer = decoder.getOutputBuffer(outIndex)
                if (outBuffer != null && bufferInfo.size > 0) {
                    val pcm = ByteArray(bufferInfo.size)
                    outBuffer.get(pcm)
                    outputCount++
                    onPcmOutput(pcm)
                }
                decoder.releaseOutputBuffer(outIndex, false)
                outIndex = decoder.dequeueOutputBuffer(bufferInfo, 0)
            }

            if (decodeCount % 50L == 0L) {
                Log.i(TAG, "OpusDecoder stats: inPackets=$decodeCount, outPcmFrames=$outputCount, inSize=${opusData.size}")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error decoding Opus packet", e)
        }
    }

    fun stop() {
        try {
            codec?.stop()
            codec?.release()
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping Opus decoder", e)
        } finally {
            codec = null
            isConfigured = false
        }
    }
}
